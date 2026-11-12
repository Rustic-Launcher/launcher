/**
 * Rustic Relay base domain: `relay.rustic-launcher.com` in production builds, the `.invalid`
 * placeholder in dev and tests. Set VITE_RELAY_DOMAIN at build time to override.
 */
export const RELAY_DOMAIN: string =
  (import.meta.env.VITE_RELAY_DOMAIN as string | undefined) ??
  (import.meta.env.PROD ? 'relay.rustic-launcher.com' : 'relay.example.invalid');

/** `<label>.<relay domain>` */
export function relayHost(label: string): string {
  return `${label}.${RELAY_DOMAIN}`;
}
