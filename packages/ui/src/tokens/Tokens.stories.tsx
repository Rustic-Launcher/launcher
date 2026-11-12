import type { Meta, StoryObj } from '@storybook/react-vite';
import { contrastRatio, palettes, PALETTE_IDS, type ColorTokens, type ThemeMode } from './palettes';
import { duration, easing, fontSize, radius } from './tokens';

const meta: Meta = {
  title: 'Foundations/Tokens',
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj;

const TEXT_PAIRS: [keyof ColorTokens, keyof ColorTokens][] = [
  ['fg', 'bg'],
  ['fgMuted', 'bg'],
  ['fgSubtle', 'surface'],
  ['accentFg', 'accent'],
  ['accentFg', 'accentHover'],
  ['accentText', 'surface2'],
  ['dangerText', 'surface'],
  ['focusRing', 'bg'],
];

const isHex = (c: string) => /^#[0-9a-f]{3,8}$/i.test(c);

function PaletteCard({ id, mode }: { id: (typeof PALETTE_IDS)[number]; mode: ThemeMode }) {
  const t = palettes[id][mode];
  return (
    <section
      className="flex flex-col gap-3 rounded-xl p-4"
      style={{ background: t.bg, color: t.fg, border: `1px solid ${t.border}` }}
    >
      <h3 className="text-sm font-semibold capitalize">
        {id} · {mode}
      </h3>
      <div className="grid grid-cols-6 gap-2">
        {(Object.keys(t) as (keyof ColorTokens)[]).map((k) => (
          <div key={k} className="flex flex-col gap-1">
            <span
              className="h-8 rounded-md"
              style={{ background: t[k], border: `1px solid ${t.border}` }}
            />
            <span className="text-2xs" style={{ color: t.fgMuted }}>
              {k}
            </span>
          </div>
        ))}
      </div>
      <ul className="flex flex-col gap-1 text-xs">
        {TEXT_PAIRS.filter(([fg, bg]) => isHex(t[fg]) && isHex(t[bg])).map(([fg, bg]) => {
          const ratio = contrastRatio(t[fg], t[bg]);
          const min = fg === 'focusRing' ? 3 : 4.5;
          return (
            <li
              key={`${fg}-${bg}`}
              className="flex items-center justify-between rounded px-2 py-1"
              style={{ background: t[bg], color: t[fg] }}
            >
              <span>
                {fg} on {bg}
              </span>
              <span className="tabular-nums">
                {ratio.toFixed(2)}:1 {ratio >= min ? 'AA' : 'fail'}
              </span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

export const Palettes: Story = {
  render: () => (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
      {PALETTE_IDS.flatMap((id) =>
        (['dark', 'light'] as ThemeMode[]).map((m) => (
          <PaletteCard key={`${id}-${m}`} id={id} mode={m} />
        )),
      )}
    </div>
  ),
};

export const Typography: Story = {
  render: () => (
    <div className="flex flex-col gap-3">
      {(Object.entries(fontSize) as [string, number][]).map(([k, px]) => (
        <div key={k} className="flex items-baseline gap-6">
          <span className="text-fg-subtle w-16 text-xs">
            {k} · {px}px
          </span>
          <span style={{ fontSize: px }}>The quick ember fox jumps over the lazy creeper</span>
        </div>
      ))}
      <div className="flex items-baseline gap-6">
        <span className="text-fg-subtle w-16 text-xs">mono</span>
        <span className="font-mono text-sm">
          [12:04:51 INFO]: Done (3.2s)! For help, type "help"
        </span>
      </div>
    </div>
  ),
};

export const RadiiAndShadows: Story = {
  render: () => (
    <div className="flex flex-col gap-8">
      <div className="flex flex-wrap gap-4">
        {(Object.entries(radius) as [string, number][]).map(([k, px]) => (
          <div
            key={k}
            className="border-border bg-surface-2 text-fg-muted grid size-24 place-items-center border text-xs"
            style={{ borderRadius: px }}
          >
            {k} · {px}px
          </div>
        ))}
      </div>
      <div className="flex flex-wrap gap-6">
        {['shadow-sm', 'shadow-md', 'shadow-lg', 'shadow-glow'].map((s) => (
          <div
            key={s}
            className={`bg-surface text-fg-muted grid size-28 place-items-center rounded-xl text-xs ${s}`}
          >
            {s}
          </div>
        ))}
        <div className="rx-glass text-fg-muted grid size-28 place-items-center rounded-xl text-xs">
          glass
        </div>
        <div className="rx-glass-strong text-fg-muted grid size-28 place-items-center rounded-xl text-xs">
          glass strong
        </div>
      </div>
    </div>
  ),
};

export const Motion: Story = {
  render: () => (
    <ul className="flex flex-col gap-2 text-sm">
      {(Object.entries(duration) as [string, number][]).map(([k, s]) => (
        <li key={k} className="flex items-center gap-4">
          <span className="text-fg-subtle w-16">{k}</span>
          <span className="tabular-nums">{s * 1000} ms</span>
        </li>
      ))}
      {(Object.entries(easing) as [string, number[]][]).map(([k, v]) => (
        <li key={k} className="flex items-center gap-4">
          <span className="text-fg-subtle w-16">ease.{k}</span>
          <span className="font-mono text-xs">cubic-bezier({v.join(', ')})</span>
        </li>
      ))}
      <li className="text-fg-muted">prefers-reduced-motion disables transitions and animations.</li>
    </ul>
  ),
};
