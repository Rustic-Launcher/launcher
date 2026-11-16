/**
 * The icon of an edition: Rustic's own mark, or the official Java / Bedrock edition icon fetched
 * at run time from minecraft.net (`edition_icon`; ADR 0172: never bundled, never committed). A
 * neutral icon shows until it loads, and stays when it can't (offline, blocked, or nothing after a
 * few seconds).
 */
import { cn } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { Box, Layers } from 'lucide-react';
import { useEffect, useState } from 'react';
import { invoke } from '@/ipc';
import { RusticMark } from '@/layout/Logo';
import type { LaunchpadEdition } from '@/stores/prefs';

function useIcon(edition: LaunchpadEdition) {
  return useQuery({
    queryKey: ['edition-icon', edition],
    queryFn: () => (edition === 'rustic' ? null : invoke('edition_icon', { edition })),
    enabled: edition !== 'rustic',
    staleTime: Infinity,
    retry: false,
  });
}

const TIMEOUT_MS = 8000;

export function EditionLogo({
  edition,
  size = 20,
  className,
}: {
  edition: LaunchpadEdition;
  size?: number;
  className?: string;
}) {
  const { data: src, isError, isSuccess } = useIcon(edition);
  const [state, setState] = useState<'loading' | 'ok' | 'failed'>('loading');
  useEffect(() => {
    if (edition === 'rustic') return;
    setState('loading');
    const timer = window.setTimeout(
      () => setState((s) => (s === 'loading' ? 'failed' : s)),
      TIMEOUT_MS,
    );
    return () => window.clearTimeout(timer);
  }, [edition]);

  if (edition === 'rustic') return <RusticMark size={size} />;
  const failed = state === 'failed' || isError || (isSuccess && !src);
  const Fallback = edition === 'java' ? Box : Layers;
  return (
    <span
      aria-hidden
      className={cn('relative inline-grid shrink-0 place-items-center', className)}
      style={{ width: size, height: size }}
    >
      {state !== 'ok' && (
        <Fallback
          data-testid={failed ? `edition-fallback-${edition}` : undefined}
          className="text-fg-muted"
          style={{ width: size * 0.85, height: size * 0.85 }}
        />
      )}
      {!failed && src && (
        <img
          src={src}
          alt=""
          width={size}
          height={size}
          referrerPolicy="no-referrer"
          draggable={false}
          data-testid={`edition-logo-${edition}`}
          onLoad={() => setState('ok')}
          onError={() => setState('failed')}
          className={cn(
            'absolute inset-0 object-contain [image-rendering:pixelated]',
            state === 'loading' && 'opacity-0',
          )}
          style={{ width: size, height: size }}
        />
      )}
    </span>
  );
}
