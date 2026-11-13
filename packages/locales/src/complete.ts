/**
 * Bundles translated completely in the launcher, the website and in game (`pt-BR`, not `pt`).
 * They get the "Translated" group of the menus and, on the website, URL prefixes and prerendered
 * pages. A bundle joins only when its three apps are complete (the apps' tests check it). Its own
 * module (`@rustic/locales/complete`) so the website's first load doesn't carry the registry.
 */
export const COMPLETE = ['en', 'fr', 'de', 'es', 'it', 'pt-BR', 'pl'] as const;
export type CompleteBundle = (typeof COMPLETE)[number];
