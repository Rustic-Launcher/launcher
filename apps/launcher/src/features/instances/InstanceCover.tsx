import { cn } from '@rustic/ui';
import { PRESET_ICONS } from '@/components/InstanceIcon';
import type { Instance } from '@/ipc/types/instances';

/** What a cover is drawn from: an id (seeds the hills) and an icon. Profiles, servers, drafts. */
export type CoverSubject = Pick<Instance, 'id' | 'icon'>;

/** A profile's colour: its preset's, else a neutral slate for picture icons. */
export function profileColor(instance: CoverSubject): string {
  return instance.icon.kind === 'preset' ? instance.icon.color : '#5b6070';
}

function seedOf(id: string): number {
  let h = 7;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) % 9973;
  return h;
}

/**
 * A profile's cover (original art drawn in code, no game asset): a sky in the profile's colour,
 * a sun, voxel hills like the catalogue's generated banners, and the profile's glyph (or its
 * picture) large on the right. Fills its positioned parent. Hosted servers and the create forms'
 * previews use it too (same `id` + `icon` shape). `wide` draws three times as many hills, for
 * banners wider than about 3:1.
 */
export function InstanceCover({
  instance,
  className,
  wide,
}: {
  instance: CoverSubject;
  className?: string;
  wide?: boolean;
}) {
  const color = profileColor(instance);
  const Glyph = instance.icon.kind === 'preset' ? PRESET_ICONS[instance.icon.preset] : null;
  const seed = seedOf(instance.id);
  const w = wide ? 480 : 160;
  const hills = Array.from({ length: w / 10 }, (_, i) => 14 + ((seed * (i + 5)) % 26));
  const far = Array.from({ length: w / 20 }, (_, i) => 30 + ((seed * (i + 2)) % 22));
  return (
    <div
      aria-hidden
      className={cn('absolute inset-0 isolate overflow-hidden', className)}
      style={{
        background: `linear-gradient(165deg, color-mix(in oklab, ${color} 78%, white) 0%, ${color} 42%, color-mix(in oklab, ${color} 30%, #07080b) 100%)`,
      }}
    >
      <svg
        viewBox={`0 0 ${w} 90`}
        preserveAspectRatio="xMidYMax slice"
        className="absolute inset-0 size-full"
      >
        <circle
          cx={(wide ? 60 : 24) + (seed % 40)}
          cy={wide ? 44 : 20}
          r="9"
          fill="white"
          fillOpacity="0.28"
        />
        {far.map((h, i) => (
          <rect
            key={`f${i}`}
            x={i * 20}
            y={90 - h}
            width="20.5"
            height={h}
            fill="#07080b"
            fillOpacity="0.16"
          />
        ))}
        {hills.map((h, i) => (
          <rect
            key={i}
            x={i * 10}
            y={90 - h}
            width="10.5"
            height={h}
            fill="#07080b"
            fillOpacity={0.4 + (i % 3) * 0.1}
          />
        ))}
        {hills.map((h, i) => (
          <rect
            key={`g${i}`}
            x={i * 10}
            y={90 - h}
            width="10.5"
            height="1.6"
            fill="white"
            fillOpacity="0.16"
          />
        ))}
      </svg>
      {Glyph ? (
        <Glyph
          className={cn(
            'absolute rotate-[-8deg] text-white/85 drop-shadow-[0_8px_22px_rgba(0,0,0,0.35)]',
            wide
              ? 'top-[8%] right-[6%] aspect-square h-[78%] w-auto'
              : 'top-[10%] right-[7%] size-[58%]',
          )}
          strokeWidth={1.3}
        />
      ) : instance.icon.kind === 'image' ? (
        <img
          src={instance.icon.url}
          alt=""
          draggable={false}
          className="absolute top-[12%] right-[8%] aspect-square h-[58%] rotate-[-6deg] rounded-2xl object-cover shadow-lg ring-2 ring-white/20"
        />
      ) : null}
    </div>
  );
}
