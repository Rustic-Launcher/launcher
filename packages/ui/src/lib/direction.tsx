import { Direction } from 'radix-ui';
import { useSyncExternalStore, type ReactNode } from 'react';

/**
 * The document's writing direction (`<html dir>`, set by each app from its language: `rtl` for
 * Arabic and Hebrew, ADR 0174), followed live: the launcher changes language without a reload.
 */
function subscribe(onChange: () => void) {
  if (typeof MutationObserver === 'undefined') return () => {};
  const mo = new MutationObserver(onChange);
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ['dir'] });
  return () => mo.disconnect();
}
const read = (): 'ltr' | 'rtl' =>
  typeof document !== 'undefined' && document.documentElement.dir === 'rtl' ? 'rtl' : 'ltr';

export function useDocumentDir(): 'ltr' | 'rtl' {
  return useSyncExternalStore(subscribe, read, () => 'ltr');
}

/**
 * Hands the document's direction to Radix (menus, sliders, tabs and scroll areas read it from
 * this provider, not from the page), so their keyboard arrows and sides follow it.
 */
export function DocumentDirection({ children }: { children: ReactNode }) {
  return <Direction.Provider dir={useDocumentDir()}>{children}</Direction.Provider>;
}
