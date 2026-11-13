/**
 * The browser mock's `rustic://content/...` and `rustic://server/...` links: the same strict rules
 * as the desktop side (`src-tauri/src/lib.rs` `content_link`, `server_link`), so the preview and
 * the e2e tests open the same pages.
 * A link reaches the mock through `localStorage['rustic.mock.deeplink']` (a "cold start": taken
 * once by `app_take_deep_link`) or `window.rusticMockDeepLink(url)` (the app already running).
 */
import type { ContentDeepLink, PageDeepLink, ServerDeepLink } from '../types/auth';
import { emit } from './runtime';

const PROJECT = /^[a-z0-9_-]{1,64}$/;
const VERSION = /^[0-9A-Za-z.+_-]{1,64}$/;

function decode(s: string): string | null {
  try {
    return decodeURIComponent(s);
  } catch {
    return null;
  }
}

/** `rustic://content/rustic/<project>[?version=<project>@<version>]`, or null when refused. */
export function parseContentLink(url: string): ContentDeepLink | null {
  const rest = url.startsWith('rustic://content/') ? url.slice('rustic://content/'.length) : null;
  if (rest === null || rest.includes('#')) return null;
  const q = rest.indexOf('?');
  let path = q < 0 ? rest : rest.slice(0, q);
  const query = q < 0 ? '' : rest.slice(q + 1);
  if (path.endsWith('/')) path = path.slice(0, -1);
  const parts = path.split('/');
  if (parts.length !== 2 || parts[0] !== 'rustic') return null;
  const project = decode(parts[1]!);
  if (project === null || !PROJECT.test(project)) return null;
  let version: string | null = null;
  if (query) {
    for (const kv of query.split('&')) {
      if (!kv.startsWith('version=') || version !== null) return null;
      const v = decode(kv.slice('version='.length));
      const at = v?.indexOf('@') ?? -1;
      if (!v || at < 0 || v.slice(0, at) !== project || !VERSION.test(v.slice(at + 1))) {
        return null;
      }
      version = v;
    }
  }
  return { action: 'content', source: 'rustic', project, version, url };
}

const SLUG = /^[a-z0-9](?:[a-z0-9-]{1,46})[a-z0-9]$/;
const LISTING_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** `rustic://server/<slug or id>` (ADR 0158), or null when refused (as `lib.rs` `server_link`). */
export function parseServerLink(url: string): ServerDeepLink | null {
  const rest = url.startsWith('rustic://server/') ? url.slice('rustic://server/'.length) : null;
  if (rest === null || rest.includes('#') || rest.includes('?')) return null;
  const path = rest.endsWith('/') ? rest.slice(0, -1) : rest;
  if (path.includes('/')) return null;
  const id = decode(path);
  if (id === null || !(SLUG.test(id) || LISTING_ID.test(id))) return null;
  return { action: 'server', listing: id, url };
}

/** A page-opening link (content or server), or null. */
export function parsePageLink(url: string): PageDeepLink | null {
  return parseContentLink(url) ?? parseServerLink(url);
}

const KEY = 'rustic.mock.deeplink';

/** The link waiting at start (once), as the desktop side keeps it until the UI takes it. */
export function takeMockDeepLink(): PageDeepLink | null {
  try {
    const url = window.localStorage.getItem(KEY);
    if (!url) return null;
    window.localStorage.removeItem(KEY);
    return parsePageLink(url);
  } catch {
    return null;
  }
}

declare global {
  interface Window {
    /** Browser mock only: a `rustic://` link arriving while the app runs. */
    rusticMockDeepLink?: (url: string) => boolean;
  }
}

if (typeof window !== 'undefined') {
  window.rusticMockDeepLink = (url: string) => {
    const link = parsePageLink(url);
    if (link) emit('deep-link', link);
    return !!link;
  };
}
