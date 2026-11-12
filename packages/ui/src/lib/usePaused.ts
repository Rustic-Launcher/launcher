import { useEffect, useState } from 'react';

/**
 * True while the app asks every animation to rest: `<html data-paused>` (the launcher sets it while
 * a game runs) or a hidden / minimised window. Loops (WebGL, timers) check it; CSS animations are
 * paused by language.css.
 */
export function useAnimationsPaused(): boolean {
  const read = () =>
    typeof document !== 'undefined' &&
    (document.hidden || document.documentElement.hasAttribute('data-paused'));
  const [paused, setPaused] = useState(read);
  useEffect(() => {
    const update = () => setPaused(read());
    const mo = new MutationObserver(update);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-paused'] });
    document.addEventListener('visibilitychange', update);
    return () => {
      mo.disconnect();
      document.removeEventListener('visibilitychange', update);
    };
  }, []);
  return paused;
}
