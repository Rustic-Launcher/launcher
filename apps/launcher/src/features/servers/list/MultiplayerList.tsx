import {
  Button,
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownSeparator,
  DropdownTrigger,
  EmptyState,
  IconButton,
  Input,
  Skeleton,
  toast,
  Tooltip,
} from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import {
  Copy,
  Globe,
  LogIn,
  MoreHorizontal,
  Pencil,
  Plus,
  RefreshCw,
  Signal,
  Star,
  Trash2,
  X,
} from 'lucide-react';
import { useMemo, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { usePlay } from '@/features/play/usePlay';
import type { MultiplayerEntry } from '@/ipc/types/multiplayer';
import { formatNumber } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import {
  copyText,
  serverKeys,
  sortMultiplayer,
  useMultiplayer,
  useMultiplayerActions,
  usePing,
} from './queries';

const ADDRESS_RE = /^[a-z0-9.-]+(:\d{1,5})?$/i;

export function pingTone(ms: number | null | undefined): 'good' | 'ok' | 'bad' | 'none' {
  if (ms == null) return 'none';
  if (ms < 60) return 'good';
  if (ms < 150) return 'ok';
  return 'bad';
}

function EntryForm({
  initial,
  onSubmit,
  onCancel,
  pending,
  submitLabel,
}: {
  initial?: { name: string; address: string };
  onSubmit: (v: { name: string; address: string }) => void;
  onCancel: () => void;
  pending: boolean;
  submitLabel: string;
}) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const [name, setName] = useState(initial?.name ?? '');
  const [address, setAddress] = useState(initial?.address ?? '');
  const [touched, setTouched] = useState(false);
  const addressBad = touched && !ADDRESS_RE.test(address.trim());
  const nameBad = touched && !name.trim();
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (!name.trim() || !ADDRESS_RE.test(address.trim())) return;
    onSubmit({ name: name.trim(), address: address.trim() });
  };
  return (
    <form onSubmit={submit} className="flex flex-wrap items-start gap-2" noValidate>
      <div className="flex min-w-48 flex-1 flex-col gap-1">
        <Input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('multiplayer.namePlaceholder')}
          aria-label={t('multiplayer.name')}
          invalid={nameBad}
        />
      </div>
      <div className="flex min-w-56 flex-[1.4] flex-col gap-1">
        <Input
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          placeholder={t('multiplayer.addressPlaceholder')}
          aria-label={t('multiplayer.address')}
          invalid={addressBad}
          className="[&_input]:font-mono"
        />
        {addressBad && (
          <p className="text-danger-text text-xs">{t('multiplayer.addressInvalid')}</p>
        )}
      </div>
      <Button type="submit" variant="primary" loading={pending}>
        {submitLabel}
      </Button>
      <Button type="button" variant="ghost" onClick={onCancel}>
        {tc('actions.cancel')}
      </Button>
    </form>
  );
}

function PingCell({ address }: { address: string }) {
  const { t, i18n } = useTranslation('servers');
  const { data, isLoading } = usePing(address);
  if (isLoading) return <Skeleton className="h-4 w-14" />;
  if (!data?.online) {
    return <span className="text-danger-text text-xs font-medium">{t('multiplayer.offline')}</span>;
  }
  const tone = pingTone(data.latency_ms);
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 text-xs tabular-nums',
        tone === 'good' && 'text-success-text',
        tone === 'ok' && 'text-warning-text',
        tone === 'bad' && 'text-danger-text',
      )}
    >
      <Signal className="size-3.5" aria-hidden />
      {t('multiplayer.ping', { ms: formatNumber(data.latency_ms ?? 0, i18n.language) })}
    </span>
  );
}

function MotdCell({ address }: { address: string }) {
  const { t, i18n } = useTranslation('servers');
  const { data, isLoading } = usePing(address);
  if (isLoading) return <Skeleton className="h-4 w-48" />;
  if (!data?.online)
    return <span className="text-fg-subtle text-xs">{t('multiplayer.unreachable')}</span>;
  return (
    <span className="text-fg-muted flex min-w-0 items-center gap-3 text-xs">
      <span className="truncate">{data.motd}</span>
      <span className="text-fg-subtle shrink-0 tabular-nums">
        {t('multiplayer.players', {
          online: formatNumber(data.players_online, i18n.language),
          max: formatNumber(data.players_max, i18n.language),
        })}
      </span>
    </span>
  );
}

function Row({ entry }: { entry: MultiplayerEntry }) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const { update, remove, add } = useMultiplayerActions();
  const [editing, setEditing] = useState(false);

  if (editing) {
    return (
      <li className="rx-surface border-accent/40 rounded-xl px-4 py-3">
        <EntryForm
          initial={entry}
          pending={update.isPending}
          submitLabel={tc('actions.save')}
          onCancel={() => setEditing(false)}
          onSubmit={(v) =>
            update.mutate({ id: entry.id, patch: v }, { onSuccess: () => setEditing(false) })
          }
        />
      </li>
    );
  }

  return (
    <li
      className="rx-surface group flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors"
      data-testid={`mp-${entry.id}`}
    >
      <button
        type="button"
        aria-pressed={entry.favorite}
        aria-label={
          entry.favorite
            ? t('multiplayer.unfavorite', { name: entry.name })
            : t('multiplayer.favorite', { name: entry.name })
        }
        onClick={() => update.mutate({ id: entry.id, patch: { favorite: !entry.favorite } })}
        className="rx-focus-ring text-fg-subtle hover:text-warning rounded-md p-1.5"
        data-testid="mp-favorite"
      >
        <Star className={cn('size-4', entry.favorite && 'fill-warning text-warning')} />
      </button>
      <span
        className="bg-surface-3 text-fg-muted grid size-10 shrink-0 place-items-center rounded-lg"
        aria-hidden
      >
        <Globe className="size-5" />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="text-fg truncate text-sm font-semibold">{entry.name}</span>
          <code className="text-fg-subtle truncate font-mono text-xs">{entry.address}</code>
        </div>
        <MotdCell address={entry.address} />
      </div>
      <div className="w-20 shrink-0 text-right">
        <PingCell address={entry.address} />
      </div>
      <Tooltip
        content={
          instance
            ? t('multiplayer.joinWith', { instance: instance.name })
            : t('multiplayer.noInstance')
        }
      >
        <span>
          <Button
            size="sm"
            variant="soft"
            leftIcon={<LogIn />}
            disabled={!instance}
            aria-label={t('multiplayer.joinAria', { name: entry.name })}
            onClick={() => {
              void play.play({ join: { address: entry.address, name: entry.name } });
              toast({ tone: 'info', title: t('multiplayer.joining', { name: entry.name }) });
            }}
          >
            {t('multiplayer.join')}
          </Button>
        </span>
      </Tooltip>
      <Dropdown>
        <DropdownTrigger asChild>
          <IconButton
            size="sm"
            tooltip={false}
            label={t('multiplayer.more', { name: entry.name })}
            icon={<MoreHorizontal />}
          />
        </DropdownTrigger>
        <DropdownContent>
          <DropdownItem icon={<Pencil />} onSelect={() => setEditing(true)}>
            {tc('actions.edit')}
          </DropdownItem>
          <DropdownItem
            icon={<Copy />}
            onSelect={() =>
              void copyText(entry.address).then(
                (ok) => ok && toast({ tone: 'success', title: tc('actions.copied') }),
              )
            }
          >
            {t('multiplayer.copyAddress')}
          </DropdownItem>
          <DropdownSeparator />
          <DropdownItem
            icon={<Trash2 />}
            destructive
            onSelect={() =>
              remove.mutate(entry.id, {
                onSuccess: () =>
                  toast({
                    title: t('multiplayer.removed', { name: entry.name }),
                    action: {
                      label: t('multiplayer.undo'),
                      onClick: () => add.mutate({ name: entry.name, address: entry.address }),
                    },
                  }),
              })
            }
          >
            {tc('actions.remove')}
          </DropdownItem>
        </DropdownContent>
      </Dropdown>
    </li>
  );
}

export function MultiplayerList({ query = '' }: { query?: string }) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const { data, isLoading, isError, error, refetch } = useMultiplayer();
  const { add } = useMultiplayerActions();
  const [adding, setAdding] = useState(false);
  const sorted = useMemo(() => {
    const q = query.trim().toLowerCase();
    return sortMultiplayer(
      (data ?? []).filter(
        (e) => !q || e.name.toLowerCase().includes(q) || e.address.toLowerCase().includes(q),
      ),
    );
  }, [data, query]);
  const favorites = sorted.filter((e) => e.favorite);
  const others = sorted.filter((e) => !e.favorite);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-fg-muted text-sm">{t('multiplayer.description')}</p>
        <div className="flex gap-2">
          <Button
            variant="ghost"
            size="sm"
            leftIcon={<RefreshCw />}
            onClick={() => void qc.invalidateQueries({ queryKey: ['multiplayer', 'ping'] })}
          >
            {t('multiplayer.refreshPings')}
          </Button>
          <Button
            size="sm"
            variant={adding ? 'ghost' : 'secondary'}
            leftIcon={adding ? <X /> : <Plus />}
            onClick={() => setAdding((a) => !a)}
            aria-expanded={adding}
          >
            {adding ? tc('actions.cancel') : t('multiplayer.add')}
          </Button>
        </div>
      </div>
      {adding && (
        <div className="rx-surface rounded-xl p-4">
          <EntryForm
            pending={add.isPending}
            submitLabel={tc('actions.add')}
            onCancel={() => setAdding(false)}
            onSubmit={(v) =>
              add.mutate(v, {
                onSuccess: () => {
                  setAdding(false);
                  toast({ tone: 'success', title: t('multiplayer.added', { name: v.name }) });
                },
                onError: (e) =>
                  toast({
                    tone: 'error',
                    title: t('multiplayer.addFailed'),
                    description: e.message,
                  }),
              })
            }
          />
        </div>
      )}
      {isLoading && <ListSkeleton rows={5} />}
      {isError && <QueryError onRetry={() => void refetch()} message={error.message} />}
      {data && data.length === 0 && (
        <EmptyState
          icon={<Globe />}
          title={t('multiplayer.empty.title')}
          description={t('multiplayer.empty.description')}
        />
      )}
      {favorites.length > 0 && (
        <section aria-labelledby="mp-fav" className="flex flex-col gap-2">
          <h3
            id="mp-fav"
            className="text-2xs text-fg-subtle font-semibold tracking-wider uppercase"
          >
            {t('multiplayer.favorites', { count: favorites.length })}
          </h3>
          <ul className="flex flex-col gap-2" data-testid="mp-favorites">
            {favorites.map((e) => (
              <Row key={e.id} entry={e} />
            ))}
          </ul>
        </section>
      )}
      {others.length > 0 && (
        <section aria-labelledby="mp-all" className="flex flex-col gap-2">
          <h3
            id="mp-all"
            className="text-2xs text-fg-subtle font-semibold tracking-wider uppercase"
          >
            {t('multiplayer.others', { count: others.length })}
          </h3>
          <ul className="flex flex-col gap-2" data-testid="mp-others">
            {others.map((e) => (
              <Row key={e.id} entry={e} />
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export { serverKeys };
