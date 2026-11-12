import {
  Badge,
  Button,
  Dialog,
  EmptyState,
  IconButton,
  SearchInput,
  SegmentedControl,
  Switch,
  Tooltip,
} from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { ArrowUpCircle, Package, Plus, Trash2, XCircle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import type { Instance, InstalledContent } from '@/ipc/types/instances';
import { initials } from '../logic';
import { useContentMutations, useInstanceContent } from '../queries';

type Filter = 'all' | 'mod' | 'shader' | 'resourcepack';

export function ContentTile({ name }: { name: string }) {
  return (
    <span
      aria-hidden
      className="bg-surface-3 text-fg-muted grid size-10 shrink-0 place-items-center rounded-lg text-xs font-bold"
    >
      {initials(name)}
    </span>
  );
}

export function ContentTab({ instance }: { instance: Instance }) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const { data, isLoading, isError, refetch } = useInstanceContent(instance.id);
  const m = useContentMutations(instance.id);
  const [filter, setFilter] = useState<Filter>('all');
  const [query, setQuery] = useState('');
  const [removing, setRemoving] = useState<InstalledContent | null>(null);

  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (data ?? [])
      .filter((c) => c.kind !== 'datapack')
      .filter((c) => filter === 'all' || c.kind === filter)
      .filter((c) => !q || c.name.toLowerCase().includes(q))
      .sort((a, b) => Number(a.compatible) - Number(b.compatible) || a.name.localeCompare(b.name));
  }, [data, filter, query]);
  const updates = (data ?? []).filter((c) => c.update_available);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <SegmentedControl
          label={t('content.filter')}
          value={filter}
          onValueChange={(v) => setFilter(v as Filter)}
          options={[
            { value: 'all', label: t('content.kinds.all') },
            { value: 'mod', label: t('content.kinds.mod') },
            { value: 'resourcepack', label: t('content.kinds.resourcepack') },
            { value: 'shader', label: t('content.kinds.shader') },
          ]}
        />
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={t('content.search')}
          aria-label={t('content.search')}
          clearLabel={tc('actions.clear')}
          className="w-60"
        />
        <div className="ml-auto flex gap-2">
          {updates.length > 0 && (
            <Button
              leftIcon={<ArrowUpCircle />}
              loading={m.update.isPending}
              onClick={() => m.update.mutate(undefined)}
            >
              {t('content.updateAll', { count: updates.length })}
            </Button>
          )}
          <Button asChild variant="primary">
            <Link to="/content" search={{ target: `instance:${instance.id}` }}>
              <Plus /> {t('content.browse')}
            </Link>
          </Button>
        </div>
      </div>
      {isLoading && <ListSkeleton rows={6} />}
      {isError && <QueryError onRetry={() => void refetch()} />}
      {data && list.length === 0 && (
        <EmptyState
          icon={<Package />}
          title={query ? t('content.noMatch') : t('content.empty.title')}
          description={query ? undefined : t('content.empty.description')}
          action={
            <Button asChild variant="primary">
              <Link to="/content" search={{ target: `instance:${instance.id}` }}>
                {t('content.browse')}
              </Link>
            </Button>
          }
        />
      )}
      {list.length > 0 && (
        <ul className="rx-surface divide-border flex flex-col divide-y overflow-hidden rounded-xl">
          {list.map((c) => (
            <li
              key={c.id}
              className="flex items-center gap-3 px-4 py-3"
              data-testid={`content-row-${c.id}`}
            >
              <ContentTile name={c.name} />
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`truncate text-sm font-semibold ${c.enabled ? '' : 'text-fg-subtle line-through'}`}
                  >
                    {c.name}
                  </span>
                  <span className="text-fg-subtle text-xs tabular-nums">{c.version}</span>
                  <Badge>{t(`content.kinds.${c.kind as Exclude<Filter, 'all'>}`)}</Badge>
                  <Badge variant="neutral">{t(`content.sources.${c.source}`)}</Badge>
                  {c.update_available && (
                    <Badge variant="info" icon={<ArrowUpCircle />}>
                      {t('content.updateTo', { version: c.update_available })}
                    </Badge>
                  )}
                  {!c.compatible && (
                    <Tooltip content={c.incompatibility ?? ''}>
                      <Badge variant="danger" icon={<XCircle />} tabIndex={0}>
                        {t('content.incompatible', { reason: c.incompatibility ?? '' })}
                      </Badge>
                    </Tooltip>
                  )}
                </div>
                <span className="text-2xs text-fg-subtle truncate font-mono">{c.filename}</span>
              </div>
              {c.update_available && (
                <IconButton
                  size="sm"
                  label={t('content.updateOne', { name: c.name })}
                  icon={<ArrowUpCircle />}
                  onClick={() => m.update.mutate(c.id)}
                />
              )}
              <Switch
                checked={c.enabled}
                disabled={!c.compatible && !c.enabled}
                onCheckedChange={(v) => m.setEnabled.mutate({ content_id: c.id, enabled: v })}
                aria-label={t('content.toggle', { name: c.name })}
              />
              <IconButton
                size="sm"
                variant="danger"
                label={t('content.remove', { name: c.name })}
                icon={<Trash2 />}
                onClick={() => setRemoving(c)}
              />
            </li>
          ))}
        </ul>
      )}
      <Dialog
        open={!!removing}
        onOpenChange={(o) => !o && setRemoving(null)}
        tone="destructive"
        icon={<Trash2 />}
        title={t('content.removeTitle', { name: removing?.name ?? '' })}
        description={t('content.removeBody')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button onClick={() => setRemoving(null)}>{tc('actions.cancel')}</Button>
            <Button
              variant="danger"
              loading={m.remove.isPending}
              onClick={() =>
                removing && m.remove.mutate(removing.id, { onSuccess: () => setRemoving(null) })
              }
            >
              {tc('actions.remove')}
            </Button>
          </>
        }
      />
    </div>
  );
}
