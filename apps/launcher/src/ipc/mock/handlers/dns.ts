/**
 * Mock DNS: Porkbun connection with the user's own API keys, domain search/purchase on the user's
 * Porkbun account, automatic configuration, and manual records + live verification for any registrar.
 */
import {
  buildRequiredRecords,
  normalizeDomain,
  relayRouteTarget,
  type RecordTarget,
} from '@/features/servers/manage/dnsLogic';
import { RELAY_DOMAIN } from '@/lib/relay';
import type {
  DnsProviderConnection,
  DnsVerification,
  DomainAvailability,
  DomainBinding,
  DomainTarget,
  Money,
  OwnedDomain,
  RecordCheckStatus,
  RequiredRecords,
} from '../../types/dns';
import { findServer } from '../data/serverAdmin';
import { serversDb } from '../data/servers';
import {
  daysAgo,
  emit,
  fail,
  latency,
  nowIso,
  persistGet,
  persistSet,
  sleep,
  SPEED,
  type MockHandlers,
} from '../runtime';

const clone = <T>(v: T): T => structuredClone(v);

let porkbun: DnsProviderConnection = persistGet<DnsProviderConnection>('porkbun', {
  provider: 'porkbun',
  connected: false,
  account_label: null,
  connected_at: null,
});

const inYears = (y: number) => new Date(Date.now() + y * 365 * 86_400_000).toISOString();

const owned: OwnedDomain[] = [
  { domain: 'samcraft.fr', expires_at: inYears(0.7), auto_renew: true, api_access: true },
  { domain: 'lescopains.gg', expires_at: inYears(1.4), auto_renew: true, api_access: true },
  { domain: 'vieux-blog.net', expires_at: inYears(0.2), auto_renew: false, api_access: false },
];

/** Porkbun quotes in USD (minor units). */
const TLD_PRICES: Record<string, [number, number]> = {
  net: [1108, 1252],
  gg: [5420, 5420],
  fr: [899, 1049],
  xyz: [199, 1298],
  com: [1057, 1132],
  fun: [249, 2860],
};
const TAKEN = new Set(['minecraft', 'google', 'test', 'example', 'rustic', 'horizon', 'samcraft']);
/** UX_SPEC: purchases over $100 are refused in the launcher (buy on porkbun.com). */
export const MAX_PRICE_USD_MINOR = 10_000;

const usd = (amount_minor: number): Money => ({ amount_minor, currency: 'USD' });

function quote(domain: string): DomainAvailability {
  const [base = '', ...rest] = domain.split('.');
  const tld = rest.join('.');
  const prices = TLD_PRICES[tld];
  const ownedAlready = owned.some((o) => o.domain === domain);
  const taken = ownedAlready || TAKEN.has(base) || (tld === 'com' && base.length < 12) || !prices;
  const premium = !taken && base.length <= 4;
  const first = premium ? 24_900 : (prices?.[0] ?? 0);
  const renew = premium ? 24_900 : (prices?.[1] ?? 0);
  return {
    domain,
    available: !taken,
    premium,
    price: taken ? null : usd(first),
    renewal_price: taken ? null : usd(renew),
  };
}

function requirePorkbun() {
  if (!porkbun.connected)
    fail('unauthorized', 'Connect your Porkbun account first', { reason: 'porkbun_not_connected' });
}

function server(id: string) {
  return findServer(id) ?? fail('not_found', `server ${id}`);
}

function recordTarget(serverId: string): RecordTarget {
  const s = server(serverId);
  if (s.hosting.mode === 'vps') {
    return {
      mode: 'vps',
      ipv4: s.hosting.host,
      ipv6: s.id === 'srv_vps' ? '2001:41d0:304:200::8a1c' : null,
      port: s.port,
    };
  }
  return { mode: s.hosting.mode, relayHostname: relayRouteTarget(serverId) };
}

function required(t: DomainTarget): RequiredRecords {
  const domain =
    normalizeDomain(t.domain) ?? fail('invalid_argument', 'domain', { reason: 'invalid_domain' });
  const sub = t.subdomain?.trim().toLowerCase() || null;
  return buildRequiredRecords(t.server_id, domain, sub, recordTarget(t.server_id));
}

/** Existing binding; seeds a verified one for servers already shared on their own domain. */
function bindingOf(serverId: string): DomainBinding | null {
  const existing = serversDb.domains[serverId];
  if (existing) return existing;
  const s = findServer(serverId);
  const addr = s?.share_address;
  if (!s || !addr || addr.endsWith(`.${RELAY_DOMAIN}`)) return null;
  const domain = owned.find((o) => addr === o.domain || addr.endsWith(`.${o.domain}`))?.domain;
  if (!domain) return null;
  const sub = addr === domain ? null : addr.slice(0, -(domain.length + 1));
  const rec = buildRequiredRecords(serverId, domain, sub, recordTarget(serverId));
  const b: DomainBinding = {
    server_id: serverId,
    domain,
    subdomain: sub,
    fqdn: rec.fqdn,
    provider: 'porkbun',
    status: 'verified',
    records: rec.records,
    verified_at: daysAgo(20),
  };
  serversDb.domains[serverId] = b;
  return b;
}

function setBinding(b: DomainBinding) {
  serversDb.domains[b.server_id] = b;
  if (b.status === 'verified') {
    const s = findServer(b.server_id);
    if (s) s.share_address = b.fqdn;
  }
  emit('dns-status', clone(b));
}

// Manual verification progresses over successive polls (per server + name).
const polls = new Map<string, number>();
const OLD_VALUES: Record<string, string> = {
  CNAME: 'parking.registrar-default.example.',
  A: '185.199.108.153',
  AAAA: '2606:50c0:8000::153',
  SRV: '0 5 25565 old-host.example.',
  TXT: 'v=spf1 include:_spf.example ~all',
};

function statusAt(stage: number, type: string, value: string) {
  let status: RecordCheckStatus;
  let found: string[] = [];
  let resolvers_ok = 0;
  if (stage <= 0) status = 'missing';
  else if (stage === 1 && (type === 'A' || type === 'CNAME')) {
    status = 'wrong_value';
    found = [OLD_VALUES[type] ?? '?'];
  } else if (stage <= 2) {
    status = 'propagating';
    resolvers_ok = stage === 1 ? 1 : 2;
    found = [value];
  } else {
    status = 'ok';
    resolvers_ok = 3;
    found = [value];
  }
  return { status, found, resolvers_ok, resolvers_total: 3 };
}

export const dnsHandlers: MockHandlers = {
  dns_porkbun_status: async () => {
    await latency(0.4);
    return clone(porkbun);
  },
  dns_connect_porkbun: async ({ api_key, secret_api_key }) => {
    await latency(3);
    const k = api_key.trim();
    const sk = secret_api_key.trim();
    if (!/^pk1_[A-Za-z0-9]{8,}$/.test(k)) fail('invalid_argument', 'api_key', { field: 'api_key' });
    if (!/^sk1_[A-Za-z0-9]{8,}$/.test(sk))
      fail('invalid_argument', 'secret_api_key', { field: 'secret_api_key' });
    if (k.toLowerCase().includes('revoked'))
      fail('unauthorized', 'Porkbun refused these keys', { reason: 'porkbun_rejected' });
    // Keys go to the OS keychain in the real backend; only a masked label comes back.
    porkbun = {
      provider: 'porkbun',
      connected: true,
      account_label: `${k.slice(0, 4)}…${k.slice(-4)}`,
      connected_at: nowIso(),
    };
    persistSet('porkbun', porkbun);
    return clone(porkbun);
  },
  dns_disconnect: async () => {
    await latency(0.6);
    porkbun = { provider: 'porkbun', connected: false, account_label: null, connected_at: null };
    persistSet('porkbun', porkbun);
    return clone(porkbun);
  },
  dns_list_domains: async () => {
    await latency();
    requirePorkbun();
    return clone(owned);
  },
  dns_check_domain: async ({ query }) => {
    await latency(1.5);
    const q = query
      .trim()
      .toLowerCase()
      .replace(/^https?:\/\//, '');
    if (!q) return [];
    const exact = q.includes('.') ? normalizeDomain(q) : null;
    const base = (exact ? exact.split('.')[0] : q.replace(/[^a-z0-9-]/g, '')) ?? '';
    if (!base) fail('invalid_argument', 'query');
    const names = [
      exact,
      ...['net', 'gg', 'fr', 'com', 'xyz', 'fun'].map((t) => `${base}.${t}`),
    ].filter((d, i, all): d is string => !!d && all.indexOf(d) === i);
    return names.map(quote);
  },
  dns_buy_domain: async (req) => {
    requirePorkbun();
    if (!req.agree_registrar_terms || !req.authorize_order)
      fail('invalid_argument', 'consent', { reason: 'consent_required' });
    await latency(4);
    const q = quote(req.domain);
    if (!q.available || !q.price)
      fail('conflict', 'domain not available', { reason: 'not_available' });
    if (q.price.amount_minor > MAX_PRICE_USD_MINOR)
      fail('invalid_argument', 'over limit', { reason: 'over_limit' });
    if (
      q.price.amount_minor !== req.expected_price.amount_minor ||
      q.price.currency !== req.expected_price.currency
    ) {
      fail('conflict', 'price changed', { reason: 'price_changed', price: q.price });
    }
    owned.push({
      domain: req.domain,
      expires_at: inYears(req.years),
      auto_renew: true,
      api_access: true,
    });
    return { order_id: `pb-${Date.now().toString(36)}`, domain: req.domain, charged: q.price };
  },
  dns_configure_server: async (t) => {
    requirePorkbun();
    await latency(2);
    const rec = required(t);
    const domain = rec.fqdn === t.domain ? t.domain : (normalizeDomain(t.domain) ?? t.domain);
    const o =
      owned.find((x) => x.domain === domain) ??
      fail('not_found', 'domain not in this Porkbun account', { reason: 'not_owned' });
    if (!o.api_access)
      fail('invalid_argument', 'API access is off for this domain', { reason: 'api_access_off' });
    const b: DomainBinding = {
      server_id: t.server_id,
      domain,
      subdomain: t.subdomain?.trim() || null,
      fqdn: rec.fqdn,
      provider: 'porkbun',
      status: 'pending',
      records: rec.records,
      verified_at: null,
    };
    polls.delete(`${t.server_id}|${rec.fqdn}`);
    setBinding(b);
    const step = (ms: number) => sleep(ms / Math.max(0.25, SPEED));
    void step(1500).then(async () => {
      if (serversDb.domains[t.server_id] !== b) return;
      b.status = 'propagating';
      setBinding(b);
      await step(4500);
      if (serversDb.domains[t.server_id] !== b) return;
      b.status = 'verified';
      b.verified_at = nowIso();
      setBinding(b);
    });
    return clone(b);
  },
  dns_status: async ({ server_id }) => {
    await latency(0.4);
    server(server_id);
    const b = bindingOf(server_id);
    return b ? clone(b) : null;
  },
  dns_required_records: async (t) => {
    await latency(0.6);
    return required(t);
  },
  dns_verify: async (t) => {
    await latency(1.2);
    const rec = required(t);
    const binding = bindingOf(t.server_id);
    const key = `${t.server_id}|${rec.fqdn}`;
    let stageBase: number;
    if (binding && binding.fqdn === rec.fqdn && binding.provider === 'porkbun') {
      stageBase = binding.status === 'verified' ? 99 : binding.status === 'propagating' ? 2 : 1;
    } else {
      const n = (polls.get(key) ?? 0) + 1;
      polls.set(key, n);
      stageBase = n * (SPEED >= 2 ? 2 : 1) - 1;
    }
    let i = 0;
    const records = rec.records.map((r) => {
      // Optional records are only reported once present; a manual user usually skips them.
      if (r.optional)
        return {
          record_id: r.id,
          status: 'missing' as const,
          found: [],
          resolvers_ok: 0,
          resolvers_total: 3,
        };
      const stage =
        binding?.provider === 'porkbun' && binding.fqdn === rec.fqdn
          ? stageBase + 1
          : stageBase - i++;
      return { record_id: r.id, ...statusAt(stage, r.type, r.value) };
    });
    const allOk = rec.records.every(
      (r) => r.optional || records.find((x) => x.record_id === r.id)?.status === 'ok',
    );
    const v: DnsVerification = { fqdn: rec.fqdn, checked_at: nowIso(), records, all_ok: allOk };
    if (allOk && (!binding || binding.fqdn !== rec.fqdn || binding.status !== 'verified')) {
      const domain = normalizeDomain(t.domain) ?? t.domain;
      setBinding({
        server_id: t.server_id,
        domain,
        subdomain: t.subdomain?.trim() || null,
        fqdn: rec.fqdn,
        provider: binding?.fqdn === rec.fqdn ? binding.provider : 'manual',
        status: 'verified',
        records: rec.records,
        verified_at: nowIso(),
      });
    }
    return v;
  },
  dns_remove_binding: async ({ server_id }) => {
    await latency();
    const b = serversDb.domains[server_id];
    delete serversDb.domains[server_id];
    if (b) polls.delete(`${server_id}|${b.fqdn}`);
    const s = findServer(server_id);
    const relay = serversDb.access[server_id]?.relay;
    if (s && b && s.share_address === b.fqdn)
      s.share_address = relay?.enabled ? relay.hostname : null;
  },
};
