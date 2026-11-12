/**
 * Pure DNS helpers: which records a server needs for a domain, and how a verification result reads.
 * The desktop backend computes the authoritative records (`dns_required_records`); the mock uses
 * `buildRequiredRecords` so the UI and the mock agree, and the UI uses the summaries.
 */
import type {
  DnsRecord,
  DnsVerification,
  RecordCheckStatus,
  RequiredRecords,
} from '@/ipc/types/dns';
import { relayHost } from '@/lib/relay';

export const DEFAULT_GAME_PORT = 25565;
export const VERIFY_LABEL = '_rustic-challenge';

/** Stable relay route target for a server, e.g. `r-7f3a.<relay domain>` (the backend assigns it). */
export function relayRouteTarget(serverId: string): string {
  return relayHost(`r-${hash(serverId).toString(16).slice(0, 4)}`);
}

function hash(s: string): number {
  let h = 2166136261;
  for (const c of s) {
    h ^= c.charCodeAt(0);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return h;
}

/** Lower-cases, trims, strips scheme/path/trailing dot. Returns null when not a plausible domain. */
export function normalizeDomain(input: string): string | null {
  let d = input.trim().toLowerCase();
  d = d
    .replace(/^[a-z]+:\/\//, '')
    .replace(/\/.*$/, '')
    .replace(/\.$/, '');
  if (!/^(?=.{3,253}$)([a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/.test(d)) return null;
  return d;
}

/** A single DNS label (subdomain part), or null if invalid. Empty input = no subdomain. */
export function normalizeSubdomain(input: string): string | null | undefined {
  const s = input.trim().toLowerCase().replace(/\.$/, '');
  if (s === '') return undefined;
  if (!/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$/.test(s))
    return null;
  return s;
}

export function fqdnOf(domain: string, subdomain?: string | null): string {
  return subdomain ? `${subdomain}.${domain}` : domain;
}

/** Host part relative to the zone apex as registrars expect it ("@" for the apex). */
export function relativeHost(fqdn: string, domain: string): string {
  if (fqdn === domain) return '@';
  return fqdn.endsWith(`.${domain}`) ? fqdn.slice(0, -(domain.length + 1)) : fqdn;
}

/** Deterministic ownership token for a server + fqdn (the backend issues a random one). */
export function verificationToken(serverId: string, fqdn: string): string {
  const h = hash(`${serverId}|${fqdn}`);
  return `rustic-verify=${h.toString(36)}${(h ^ 0x5bd1e995).toString(36)}`;
}

export type RecordTarget =
  | { mode: 'local' | 'cloud'; relayHostname: string; relayPort?: number }
  | { mode: 'vps'; ipv4: string; ipv6?: string | null; port: number };

/**
 * Records for a domain:
 * - local/cloud: CNAME to the relay route, SRV `_minecraft._tcp` (optional next to a CNAME on the
 *   default port), TXT ownership. The apex cannot hold a CNAME, so an apex domain needs the SRV.
 * - vps: A (+ AAAA) to the VPS, SRV only when the port is not 25565, TXT ownership.
 */
export function buildRequiredRecords(
  serverId: string,
  domain: string,
  subdomain: string | null | undefined,
  target: RecordTarget,
): RequiredRecords {
  const fqdn = fqdnOf(domain, subdomain);
  const records: DnsRecord[] = [];
  const add = (r: Omit<DnsRecord, 'id' | 'host'>) =>
    records.push({
      ...r,
      id: `${r.type.toLowerCase()}-${records.length}`,
      host: relativeHost(r.fqdn, domain),
    });

  if (target.mode === 'vps') {
    add({ type: 'A', fqdn, value: target.ipv4, ttl: 600, purpose: 'routing' });
    if (target.ipv6) add({ type: 'AAAA', fqdn, value: target.ipv6, ttl: 600, purpose: 'routing' });
    if (target.port !== DEFAULT_GAME_PORT) {
      add({
        type: 'SRV',
        fqdn: `_minecraft._tcp.${fqdn}`,
        value: `0 5 ${target.port} ${fqdn}.`,
        priority: 0,
        weight: 5,
        port: target.port,
        ttl: 600,
        purpose: 'srv',
      });
    }
  } else {
    const port = target.relayPort ?? DEFAULT_GAME_PORT;
    if (fqdn !== domain)
      add({ type: 'CNAME', fqdn, value: `${target.relayHostname}.`, ttl: 600, purpose: 'routing' });
    add({
      type: 'SRV',
      fqdn: `_minecraft._tcp.${fqdn}`,
      value: `0 5 ${port} ${target.relayHostname}.`,
      priority: 0,
      weight: 5,
      port,
      ttl: 600,
      purpose: 'srv',
      optional: fqdn !== domain && port === DEFAULT_GAME_PORT,
    });
  }
  add({
    type: 'TXT',
    fqdn: `${VERIFY_LABEL}.${fqdn}`,
    value: verificationToken(serverId, fqdn),
    ttl: 600,
    purpose: 'ownership',
  });
  return { fqdn, hosting_mode: target.mode, records };
}

export interface VerificationSummary {
  ok: number;
  total: number;
  /** Worst status present, for the panel headline. */
  worst: RecordCheckStatus;
  connected: boolean;
}

const severity: Record<RecordCheckStatus, number> = {
  ok: 0,
  propagating: 1,
  wrong_value: 2,
  missing: 3,
};

export function summarizeVerification(
  v: DnsVerification | undefined,
  expected: number,
): VerificationSummary {
  if (!v) return { ok: 0, total: expected, worst: 'missing', connected: false };
  const ok = v.records.filter((r) => r.status === 'ok').length;
  const worst = v.records.reduce<RecordCheckStatus>(
    (w, r) => (severity[r.status] > severity[w] ? r.status : w),
    'ok',
  );
  const total = Math.max(expected, v.records.length);
  return { ok, total, worst: v.records.length === 0 ? 'missing' : worst, connected: v.all_ok };
}

/** True when every required (non-optional) record is OK. */
export function requiredOk(records: DnsRecord[], v: DnsVerification | undefined): boolean {
  if (!v) return false;
  return records
    .filter((r) => !r.optional)
    .every((r) => v.records.find((x) => x.record_id === r.id)?.status === 'ok');
}

export type Registrar =
  | 'ovh'
  | 'gandi'
  | 'namecheap'
  | 'godaddy'
  | 'cloudflare'
  | 'ionos'
  | 'hostinger'
  | 'porkbun'
  | 'generic';
export const REGISTRARS: Registrar[] = [
  'ovh',
  'gandi',
  'namecheap',
  'godaddy',
  'cloudflare',
  'ionos',
  'hostinger',
  'porkbun',
  'generic',
];

/**
 * Some registrars want the full name ("play.example.com.") in the host field, most want the relative
 * part. Only Cloudflare and Namecheap accept "@" and relative hosts the same way; Gandi/OVH want
 * the relative host with no trailing dot. We show the relative host and mention the exceptions.
 */
export function hostForRegistrar(r: DnsRecord, registrar: Registrar): string {
  if (registrar === 'ovh' && r.host === '@') return '';
  return r.host;
}
