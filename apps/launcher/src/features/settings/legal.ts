/** Product links: the Rustic website (apps/web), overridable at build time with VITE_WEB_BASE_URL. */
export const WEB_BASE: string =
  (import.meta.env.VITE_WEB_BASE_URL as string | undefined) ?? 'https://rustic-launcher.com';
const BASE = WEB_BASE;

/** The cosmetics store: the website's, opened in the browser (no store page in the launcher). */
export const STORE_URL = `${BASE}/store`;

/** A website page for an app path the launcher no longer has (`/store...` from notifications). */
export function siteUrlFor(href: string): string | null {
  return href === '/store' || href.startsWith('/store/') || href.startsWith('/store?')
    ? `${BASE}${href}`
    : null;
}

export const LEGAL_URLS = {
  privacy: `${BASE}/privacy`,
  terms: `${BASE}/terms`,
  legalNotice: `${BASE}/legal`,
  termsOfSale: `${BASE}/terms-of-sale`,
  serverDocs: `${BASE}/server`,
  fairPlay: `${BASE}/fair-play`,
  thirdParty: `${BASE}/third-party-notices`,
  horizon: 'https://github.com/horizon-rust/horizon',
  javaEdition: 'https://www.minecraft.net/store/minecraft-java-bedrock-edition-pc',
} as const;

/** Minimum age to create a Rustic account (user decision 2026-11-10). */
export const MIN_ACCOUNT_AGE = 13;
/** Digital consent age in France (CNIL). */
export const DIGITAL_CONSENT_AGE = 15;
/** Offline profiles keep working this long without a successful ownership revalidation. */
export const OFFLINE_GRACE_DAYS = 30;
/** Account deletion grace period. */
export const DELETION_GRACE_DAYS = 30;
