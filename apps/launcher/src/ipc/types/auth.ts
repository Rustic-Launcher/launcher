import type { IsoDate } from './app';

export interface Ownership {
  /** Result of the last Microsoft entitlement check. */
  owns_game: boolean;
  status: 'valid' | 'expired' | 'failed' | 'never_checked';
  checked_at: IsoDate | null;
  /** Offline profiles stop working after this date without a successful revalidation. */
  next_check_at: IsoDate | null;
}

export interface Account {
  id: string;
  /** Microsoft accounts only: offline play is the same account under its official name (ADR 0146). */
  kind: 'microsoft';
  username: string;
  /** Game profile UUID. */
  uuid: string;
  active: boolean;
  /** The last ownership check; offline play stays open until `next_check_at`. */
  ownership: Ownership | null;
  /** Microsoft accounts: the profile's current skin (https://textures.minecraft.net/...). */
  skin?: { url: string; slim: boolean } | null;
}

export interface MicrosoftLoginStart {
  flow_id: string;
  verification_uri: string;
  user_code: string;
  expires_in_s: number;
}

export interface MicrosoftLoginPoll {
  status: 'pending' | 'success' | 'expired' | 'denied' | 'no_ownership';
  account?: Account;
  /** Human-readable failure detail (developer-facing). */
  error?: string;
}

/** Browser sign-in (authorization code + PKCE, loopback redirect); polled like the device code. */
export interface MicrosoftBrowserLoginStart {
  flow_id: string;
  authorize_url: string;
}

/** Payload of the `deep-link` event (`rustic://join/<target>`, `rustic://auth?ticket=…`). */
export type DeepLinkEvent =
  | { action: 'join'; target: string; url: string }
  | { action: 'auth'; ok: boolean; error?: string | null };

/** Rustic platform identity (friends, cosmetics, purchases). Never replaces the game licence. */
export interface RusticSession {
  signed_in: boolean;
  user_id: string | null;
  username: string | null;
  email: string | null;
}

export interface RusticSignupRequest {
  email: string;
  password: string;
  username: string;
  /** YYYY-MM-DD */
  date_of_birth: string;
  /** ISO 3166-1 alpha-2 */
  country: string;
  locale?: string | null;
  /** "I accept the privacy policy" (required, versioned by the backend). */
  accept_privacy: boolean;
}

export interface RusticSignupResult {
  email_verification_required: boolean;
  guardian_required: boolean;
}

export interface RusticLoginResult {
  mfa_required: boolean;
  session: RusticSession;
}
