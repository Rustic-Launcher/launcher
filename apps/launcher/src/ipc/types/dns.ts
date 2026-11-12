import type { IsoDate } from './app';

export type Currency = 'EUR' | 'USD' | 'GBP' | 'CHF';

export interface Money {
  /** Minor units (cents). */
  amount_minor: number;
  currency: Currency;
}

export interface DnsProviderConnection {
  provider: 'porkbun';
  connected: boolean;
  /** e.g. masked API key "pk1_…3f9a". */
  account_label: string | null;
  connected_at: IsoDate | null;
}

export interface OwnedDomain {
  domain: string;
  expires_at: IsoDate;
  auto_renew: boolean;
  /** Porkbun's API access must be enabled per domain. */
  api_access: boolean;
}

export interface DomainAvailability {
  domain: string;
  available: boolean;
  premium: boolean;
  /** First-year price, as quoted by the registrar. */
  price: Money | null;
  renewal_price: Money | null;
}

export interface BuyDomainRequest {
  domain: string;
  years: number;
  /** The exact price shown to the user; the backend refuses if the quote changed. */
  expected_price: Money;
  /** Explicit tick: Porkbun's registration agreement and terms (LEGAL.md, Porkbun). */
  agree_registrar_terms: true;
  /** Explicit tick: Rustic only sends this order to Porkbun on the user's behalf. */
  authorize_order: true;
}

export interface BuyDomainResult {
  order_id: string;
  domain: string;
  charged: Money;
}

export type DnsRecordType = 'CNAME' | 'SRV' | 'A' | 'AAAA' | 'TXT';

export interface DnsRecord {
  id: string;
  type: DnsRecordType;
  /** Host part as most registrars expect it ("play", "_minecraft._tcp.play", "@"). */
  host: string;
  fqdn: string;
  value: string;
  ttl: number;
  priority?: number;
  weight?: number;
  port?: number;
  purpose: 'routing' | 'srv' | 'ownership';
  /** Not required for the address to work (e.g. SRV next to a CNAME on the default port). */
  optional?: boolean;
}

export interface RequiredRecords {
  fqdn: string;
  hosting_mode: 'local' | 'vps' | 'cloud';
  records: DnsRecord[];
}

export type RecordCheckStatus = 'missing' | 'wrong_value' | 'propagating' | 'ok';

export interface DnsVerification {
  fqdn: string;
  checked_at: IsoDate;
  records: {
    record_id: string;
    status: RecordCheckStatus;
    found: string[];
    /** Public resolvers that already return the expected value (e.g. 1 of 3 while propagating). */
    resolvers_ok?: number;
    resolvers_total?: number;
  }[];
  all_ok: boolean;
}

export interface DomainBinding {
  server_id: string;
  domain: string;
  subdomain: string | null;
  fqdn: string;
  provider: 'porkbun' | 'manual';
  status: 'pending' | 'propagating' | 'verified' | 'error';
  records: DnsRecord[];
  error?: string;
  verified_at: IsoDate | null;
}

export interface DomainTarget {
  server_id: string;
  domain: string;
  subdomain?: string | null;
}
