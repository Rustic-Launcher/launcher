import { Badge, Button, EmptyState, IconButton, Switch } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { Reorder, useDragControls } from 'motion/react';
import { ArrowDown, ArrowUp, GripVertical, Layers } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import type { InstalledContent } from '@/ipc/types/instances';
import { moveItem, sameOrder } from '../logic';
import { useContentMutations, useInstanceContent } from '../queries';
import { ContentTile } from './ContentTab';

/**
 * Resource pack load order. Top = highest priority. Drag with the handle, or use the up/down
 * buttons (keyboard). Each change is saved immediately and announced to screen readers.
 */
export function ResourcePackOrder({ instanceId }: { instanceId: string }) {
  const { t } = useTranslation('instances');
  const { data, isLoading, isError, refetch } = useInstanceContent(instanceId, 'resourcepack');
  const m = useContentMutations(instanceId);
  const [items, setItems] = useState<InstalledContent[]>([]);
  const [announce, setAnnounce] = useState('');
  const committed = useRef<InstalledContent[]>([]);

  useEffect(() => {
    if (!data) return;
    const sorted = [...data].sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
    committed.current = sorted;
    setItems(sorted);
  }, [data]);

  const commit = (next: InstalledContent[], moved?: InstalledContent) => {
    if (sameOrder(next, committed.current)) return;
    committed.current = next;
    m.reorder.mutate(next.map((p) => p.id));
    if (moved) {
      setAnnounce(
        t('packs.moved', {
          name: moved.name,
          position: next.findIndex((p) => p.id === moved.id) + 1,
          total: next.length,
        }),
      );
    }
  };

  const move = (index: number, delta: number) => {
    const next = moveItem(items, index, index + delta);
    setItems(next);
    commit(next, items[index]);
  };

  if (isLoading) return <ListSkeleton rows={3} />;
  if (isError) return <QueryError onRetry={() => void refetch()} />;
  if (items.length === 0) {
    return (
      <EmptyState
        icon={<Layers />}
        title={t('packs.empty.title')}
        description={t('packs.empty.description')}
        action={
          <Button asChild variant="primary">
            <Link to="/content" search={{ kind: 'resourcepack', target: `instance:${instanceId}` }}>
              {t('packs.browse')}
            </Link>
          </Button>
        }
      />
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-fg-muted text-sm">{t('packs.hint')}</p>
        <Button asChild size="sm">
          <Link to="/content" search={{ kind: 'resourcepack', target: `instance:${instanceId}` }}>
            {t('packs.browse')}
          </Link>
        </Button>
      </div>
      <Reorder.Group
        axis="y"
        values={items}
        onReorder={setItems}
        className="flex flex-col gap-2"
        aria-label={t('packs.listLabel')}
        data-testid="pack-order"
      >
        {items.map((p, i) => (
          <PackRow
            key={p.id}
            pack={p}
            index={i}
            total={items.length}
            onMove={(d) => move(i, d)}
            onDragEnd={() => commit(items, p)}
            onToggle={(v) => m.setEnabled.mutate({ content_id: p.id, enabled: v })}
          />
        ))}
      </Reorder.Group>
      <div aria-live="polite" className="sr-only">
        {announce}
      </div>
    </div>
  );
}

function PackRow({
  pack,
  index,
  total,
  onMove,
  onDragEnd,
  onToggle,
}: {
  pack: InstalledContent;
  index: number;
  total: number;
  onMove: (delta: number) => void;
  onDragEnd: () => void;
  onToggle: (v: boolean) => void;
}) {
  const { t } = useTranslation('instances');
  const controls = useDragControls();
  return (
    <Reorder.Item
      value={pack}
      dragListener={false}
      dragControls={controls}
      onDragEnd={onDragEnd}
      className="border-border bg-surface flex items-center gap-3 rounded-xl border px-3 py-2.5 shadow-sm"
      whileDrag={{ scale: 1.01, boxShadow: 'var(--rl-shadow-lg)' }}
      data-testid={`pack-${pack.id}`}
    >
      <button
        type="button"
        aria-hidden
        tabIndex={-1}
        onPointerDown={(e) => controls.start(e)}
        className="text-fg-subtle hover:text-fg cursor-grab touch-none rounded p-1 active:cursor-grabbing"
      >
        <GripVertical className="size-4" />
      </button>
      <span className="text-fg-subtle w-6 text-center text-sm font-semibold tabular-nums">
        {index + 1}
      </span>
      <ContentTile name={pack.name} />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-sm font-semibold">{pack.name}</span>
        <span className="text-fg-subtle truncate text-xs">
          {pack.version} · {pack.filename}
        </span>
      </div>
      {index === 0 && <Badge variant="accent">{t('packs.top')}</Badge>}
      <IconButton
        size="sm"
        label={t('packs.up', { name: pack.name })}
        icon={<ArrowUp />}
        disabled={index === 0}
        onClick={() => onMove(-1)}
      />
      <IconButton
        size="sm"
        label={t('packs.down', { name: pack.name })}
        icon={<ArrowDown />}
        disabled={index === total - 1}
        onClick={() => onMove(1)}
      />
      <Switch
        checked={pack.enabled}
        onCheckedChange={onToggle}
        aria-label={t('content.toggle', { name: pack.name })}
      />
    </Reorder.Item>
  );
}
