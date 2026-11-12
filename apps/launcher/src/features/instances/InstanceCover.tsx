import { cn } from '@rustic/ui';
import { PRESET_ICONS } from '@/components/InstanceIcon';
import type { Instance } from '@/ipc/types/instances';
import { shortVersion } from '@/lib/format';

/**
 * Portrait cover for an instance (original art drawn in code): the instance colour, a large
 * outline glyph of its icon, a voxel horizon and the Horizon build stamped like a title card.
 */
export function InstanceCover({ instance, className }: { instance: Instance; className?: string }) {
  const color = instance.icon.kind === 'preset' ? instance.icon.color : '#5b6070';
  const Glyph = instance.icon.kind === 'preset' ? PRESET_ICONS[instance.icon.preset] : null;
  const seed = instance.id.split('').reduce((a, c) => a + c.charCodeAt(0), 0);
  const blocks = Array.from({ length: 10 }, (_, i) => ({
    x: i * 10,
    h: 8 + ((seed * (i + 3)) % 17),
  }));
  return (
    <div
      aria-hidden
      className={cn('relative isolate overflow-hidden', className)}
      style={{
        background: `linear-gradient(165deg, color-mix(in oklab, ${color} 85%, white) 0%, ${color} 38%, color-mix(in oklab, ${color} 35%, #07080b) 100%)`,
      }}
    >
      <svg viewBox="0 0 100 133" preserveAspectRatio="none" className="absolute inset-0 size-full">
        {blocks.map((b, i) => (
          <rect
            key={i}
            x={b.x}
            y={133 - b.h}
            width="10.5"
            height={b.h}
            fill="#07080b"
            fillOpacity={0.35 + (i % 3) * 0.08}
          />
        ))}
        {blocks.map((b, i) => (
          <rect
            key={`g${i}`}
            x={b.x}
            y={133 - b.h}
            width="10.5"
            height="2.2"
            fill="white"
            fillOpacity="0.18"
          />
        ))}
      </svg>
      {Glyph && (
        <Glyph
          className="absolute top-1/2 left-1/2 size-[46%] -translate-x-1/2 -translate-y-[62%] text-white/90 drop-shadow-[0_6px_18px_rgba(0,0,0,0.35)]"
          strokeWidth={1.4}
        />
      )}
      <div className="absolute inset-x-0 top-3 flex flex-col items-center text-white">
        <span className="text-[10px] font-black tracking-[0.28em] uppercase opacity-80">
          Horizon
        </span>
        <span className="text-lg leading-5 font-black tracking-tight drop-shadow">
          {shortVersion(instance.horizon_version)}
        </span>
      </div>
    </div>
  );
}
