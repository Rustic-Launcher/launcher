import { cn } from '@rustic/ui';
import { Copy, Minus, Square, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { isNative } from '@/ipc';

type Win = Awaited<ReturnType<typeof getWin>>;
async function getWin() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  return getCurrentWindow();
}

/**
 * Minimise / maximise / close for the frameless Tauri windows (`decorations: false`). Hidden in
 * the browser build. `onClose` overrides closing (e.g. the in-app overlay).
 */
export function WindowControls({
  onClose,
  className,
}: {
  onClose?: () => void;
  className?: string;
}) {
  const { t } = useTranslation();
  const native = isNative();
  const [win, setWin] = useState<Win | null>(null);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!native) return;
    let off: (() => void) | undefined;
    void getWin().then(async (w) => {
      setWin(w);
      setMaximized(await w.isMaximized());
      off = await w.onResized(async () => setMaximized(await w.isMaximized()));
    });
    return () => off?.();
  }, [native]);

  if (!native && !onClose) return null;
  const btn =
    'grid h-8 w-10 place-items-center rounded-md text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg [&_svg]:size-4';
  return (
    <div className={cn('flex items-center', className)}>
      {native && (
        <>
          <button
            type="button"
            aria-label={t('window.minimize')}
            className={btn}
            onClick={() => void win?.minimize()}
          >
            <Minus />
          </button>
          <button
            type="button"
            aria-label={maximized ? t('window.restore') : t('window.maximize')}
            className={btn}
            onClick={() => void win?.toggleMaximize()}
          >
            {maximized ? <Copy className="size-3.5!" /> : <Square className="size-3.5!" />}
          </button>
        </>
      )}
      <button
        type="button"
        aria-label={t('window.close')}
        className={cn(btn, 'hover:bg-danger hover:text-white')}
        onClick={() => (onClose ? onClose() : void win?.close())}
      >
        <X />
      </button>
    </div>
  );
}
