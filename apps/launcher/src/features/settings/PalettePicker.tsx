/** Palette swatch cards with a live mini preview (radio group). */
import { cn, PALETTE_IDS, palettes, resolveTheme, type PaletteId } from '@rustic/ui';
import { Check } from 'lucide-react';
import { useRef, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { usePrefs } from '@/stores/prefs';

export function PalettePicker({ className }: { className?: string }) {
  const { t } = useTranslation('settings');
  const palette = usePrefs((s) => s.palette);
  const setPalette = usePrefs((s) => s.setPalette);
  const theme = usePrefs((s) => s.theme);
  const mode = resolveTheme(theme);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  const onKey = (e: KeyboardEvent, i: number) => {
    const n = PALETTE_IDS.length;
    let next = -1;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (i + 1) % n;
    if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (i - 1 + n) % n;
    if (next >= 0) {
      e.preventDefault();
      const id = PALETTE_IDS[next]!;
      setPalette(id);
      refs.current[next]?.focus();
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label={t('appearance.palette.label')}
      className={cn('grid grid-cols-1 gap-3 sm:grid-cols-3', className)}
    >
      {PALETTE_IDS.map((id: PaletteId, i) => {
        const c = palettes[id][mode];
        const selected = palette === id;
        return (
          <button
            key={id}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            role="radio"
            aria-checked={selected}
            tabIndex={selected ? 0 : -1}
            data-testid={`palette-${id}`}
            data-palette-id={id}
            onClick={() => setPalette(id)}
            onKeyDown={(e) => onKey(e, i)}
            className={cn(
              'rx-focus-ring group relative flex flex-col gap-3 rounded-2xl border p-3 text-left transition-[border-color,transform] duration-200 hover:-translate-y-0.5',
              selected ? 'border-accent' : 'border-border hover:border-border-strong/60',
            )}
          >
            <span
              aria-hidden
              className="relative flex h-24 overflow-hidden rounded-xl border"
              style={{ background: c.bg, borderColor: c.border }}
            >
              <span
                className="flex w-6 flex-col items-center gap-1.5 pt-2"
                style={{ background: c.bgElevated }}
              >
                <span className="size-3 rounded" style={{ background: c.accent }} />
                <span className="size-2.5 rounded" style={{ background: c.surface3 }} />
                <span className="size-2.5 rounded" style={{ background: c.surface3 }} />
              </span>
              <span className="flex flex-1 flex-col gap-1.5 p-2">
                <span className="h-2 w-16 rounded-full" style={{ background: c.fg }} />
                <span className="h-1.5 w-24 rounded-full" style={{ background: c.fgMuted }} />
                <span
                  className="mt-auto flex items-center justify-between rounded-md p-1.5"
                  style={{ background: c.surface }}
                >
                  <span className="h-1.5 w-10 rounded-full" style={{ background: c.fgSubtle }} />
                  <span
                    className="rounded px-2 py-0.5 text-[9px] font-bold uppercase"
                    style={{ background: c.accent, color: c.accentFg }}
                  >
                    {t('appearance.palette.play')}
                  </span>
                </span>
              </span>
            </span>
            <span className="flex items-center gap-2">
              <span
                className="size-3.5 rounded-full"
                style={{ background: palettes[id].swatch }}
                aria-hidden
              />
              <span className="flex flex-1 flex-col">
                <span className="text-fg text-sm font-semibold">
                  {t(`appearance.palette.names.${id}`)}
                </span>
                <span className="text-fg-muted text-xs">
                  {t(`appearance.palette.descriptions.${id}`)}
                </span>
              </span>
              {selected && (
                <span className="bg-accent text-accent-fg grid size-5 place-items-center rounded-full">
                  <Check className="size-3" strokeWidth={3} aria-hidden />
                </span>
              )}
            </span>
            {id === 'ember' && (
              <span className="text-2xs absolute top-2 right-2 rounded-full bg-black/45 px-2 py-0.5 font-medium text-white">
                {t('appearance.palette.default')}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
