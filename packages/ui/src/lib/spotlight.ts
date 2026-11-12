/**
 * One delegated pointer listener for every `.rx-surface` (language.css): the surface under the
 * pointer gets `--mx` / `--my` (its spotlight); the previous one is switched off. Fine pointers only,
 * nothing with reduced motion. Returns the cleanup.
 */
export function installSurfaceSpotlight(root: Document | HTMLElement = document): () => void {
  if (
    typeof window === 'undefined' ||
    !window.matchMedia?.('(hover: hover) and (pointer: fine)').matches ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
    return () => {};
  let lit: HTMLElement | null = null;
  let raf = 0;
  const off = () => {
    lit?.style.removeProperty('--mx');
    lit?.style.removeProperty('--my');
    lit = null;
  };
  const onMove = (e: Event) => {
    const ev = e as PointerEvent;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      const el = (ev.target as Element | null)?.closest?.<HTMLElement>('.rx-surface') ?? null;
      if (el !== lit) off();
      if (!el) return;
      const r = el.getBoundingClientRect();
      el.style.setProperty('--mx', `${ev.clientX - r.left}px`);
      el.style.setProperty('--my', `${ev.clientY - r.top}px`);
      lit = el;
    });
  };
  root.addEventListener('pointermove', onMove, { passive: true });
  root.addEventListener('pointerleave', off);
  return () => {
    cancelAnimationFrame(raf);
    root.removeEventListener('pointermove', onMove);
    root.removeEventListener('pointerleave', off);
    off();
  };
}
