import {
  Badge,
  Button,
  cn,
  EmptyState,
  IconButton,
  Input,
  SegmentedControl,
  Skeleton,
  SwitchRow,
  toast,
  Tooltip,
} from '@rustic/ui';
import type { PlayerAnimation } from '@rustic/ui/three';
import { Check, Info, Play, RotateCcw, Save, Shirt, ShoppingBag, Trash2, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Cosmetic, CosmeticSlot, Loadout } from '@/cloud/types';
import { Page, PageHeader } from '@/components/Page';
import { STORE_URL } from '@/features/settings/legal';
import { invoke } from '@/ipc';
import { QueryError } from '@/components/RouteError';
import { useAccounts } from '@/queries/core';
import { useCosmetics, useEquip, useLoadout, useOutfitActions, useOutfits } from '@/queries/store';
import {
  emoteAnimation,
  equip,
  equippedCount,
  equippedNametag,
  isEquipped,
  ownedOnly,
  sameLoadout,
  unequip,
  visualsFor,
} from './loadout';
import { useLockerPrefs } from './lockerPrefs';
import { CosmeticArt, NametagPreview, RARITY_CLASS, Viewer } from './Viewer';

/** Chip order from the UX spec (no Capes category). */
const CHIPS: ('all' | CosmeticSlot)[] = [
  'all',
  'head',
  'back',
  'top',
  'shoulder',
  'nametag',
  'emote',
  'companion',
  'trail',
  'aura',
];

const MAX_OUTFITS = 10;

export function LockerPage() {
  const { t } = useTranslation('locker');
  const cosmetics = useCosmetics();
  const loadout = useLoadout();
  const equipM = useEquip();
  const [chip, setChip] = useState<'all' | CosmeticSlot>('all');
  const [hovered, setHovered] = useState<Cosmetic | null>(null);
  const [animation, setAnimation] = useState<PlayerAnimation>('idle');
  const [emotePlaying, setEmotePlaying] = useState<PlayerAnimation | null>(null);
  const [resetKey, setResetKey] = useState(0);
  const prefs = useLockerPrefs();
  const { data: accounts } = useAccounts();
  const username = accounts?.find((a) => a.active)?.username ?? 'Player';

  const all = useMemo(() => cosmetics.data ?? [], [cosmetics.data]);
  const owned = useMemo(() => all.filter((c) => c.owned), [all]);
  const current: Loadout = loadout.data ?? {};
  const visible = chip === 'all' ? owned : owned.filter((c) => c.slot === chip);

  // Hover / focus = try on.
  const preview: Loadout = hovered && hovered.slot !== 'emote' ? equip(current, hovered) : current;
  let visuals = visualsFor(preview, all);
  if (prefs.hideTrailsAuras)
    visuals = visuals.filter((v) => v.slot !== 'trail' && v.slot !== 'aura');
  const hoveredEmote = hovered?.slot === 'emote' ? emoteAnimation(hovered) : null;
  const anim = hoveredEmote ?? emotePlaying ?? animation;
  const tag = equippedNametag(preview, all);
  const equippedEmote = all.find((c) => c.id === current.emote);

  const setLoadout = (next: Loadout, message: string) => {
    equipM.mutate(next, {
      onSuccess: () => toast({ tone: 'success', title: message, duration: 2000 }),
    });
  };

  const onTile = (c: Cosmetic) => {
    if (isEquipped(current, c))
      setLoadout(unequip(current, c.slot), t('toast.unequipped', { name: c.name }));
    else setLoadout(equip(current, c), t('toast.equipped', { name: c.name }));
  };

  const playEmote = () => {
    const a = emoteAnimation(equippedEmote);
    if (!a) return;
    setEmotePlaying(a);
    window.setTimeout(() => setEmotePlaying(null), 3200);
  };

  const loading = cosmetics.isLoading || loadout.isLoading;

  return (
    <Page wide>
      <PageHeader
        title={t('heading')}
        description={t('subtitle')}
        actions={
          <Tooltip content={t('horizonNote.detail')}>
            <Badge
              variant="info"
              size="md"
              icon={<Info />}
              tabIndex={0}
              data-testid="horizon-cosmetics-note"
            >
              {t('horizonNote.badge')}
            </Badge>
          </Tooltip>
        }
      />

      {cosmetics.isError || loadout.isError ? (
        <QueryError
          onRetry={() => {
            void cosmetics.refetch();
            void loadout.refetch();
          }}
        />
      ) : !loading && owned.length === 0 ? (
        <EmptyState
          icon={<Shirt />}
          title={t('empty.title')}
          description={t('empty.description')}
          action={
            <Button
              variant="primary"
              onClick={() => void invoke('app_open_url', { url: STORE_URL })}
              data-testid="locker-open-store"
            >
              <ShoppingBag />
              {t('empty.browse')}
            </Button>
          }
        />
      ) : (
        <div className="grid gap-6 lg:grid-cols-[minmax(320px,5fr)_7fr]">
          {/* Player */}
          <section
            aria-label={t('viewer.region')}
            className="flex flex-col gap-3 lg:sticky lg:top-0 lg:self-start"
          >
            <div className="border-border relative h-[540px] overflow-hidden rounded-2xl border bg-[radial-gradient(70%_60%_at_50%_35%,var(--rl-accent-soft),transparent_70%)]">
              {loading ? (
                <Skeleton className="size-full" />
              ) : (
                <Viewer
                  label={t('viewer.label', { count: equippedCount(current) })}
                  cosmetics={prefs.showMine ? visuals : []}
                  animation={anim}
                  resetKey={resetKey}
                />
              )}
              {tag && prefs.showMine && (
                <NametagPreview
                  name={username}
                  tag={tag}
                  className="pointer-events-none absolute top-6 left-1/2 -translate-x-1/2"
                />
              )}
              <p className="text-fg-subtle pointer-events-none absolute bottom-3 left-1/2 w-max -translate-x-1/2 text-xs">
                {t('viewer.hint')}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <SegmentedControl
                size="sm"
                label={t('viewer.animation')}
                value={animation}
                onValueChange={(v) => setAnimation(v as PlayerAnimation)}
                options={(['idle', 'walk', 'wave', 'dance'] as const).map((a) => ({
                  value: a,
                  label: t(`animations.${a}`),
                }))}
              />
              <Button
                size="sm"
                leftIcon={<Play />}
                onClick={playEmote}
                disabled={!equippedEmote}
                data-testid="play-emote"
              >
                {equippedEmote
                  ? t('viewer.playEmote', { name: equippedEmote.name })
                  : t('viewer.noEmote')}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<RotateCcw />}
                onClick={() => setResetKey((k) => k + 1)}
              >
                {t('viewer.reset')}
              </Button>
            </div>
          </section>

          {/* Items */}
          <section aria-label={t('items.region')} className="flex min-w-0 flex-col gap-4">
            <div role="group" aria-label={t('categories.label')} className="flex flex-wrap gap-1.5">
              {CHIPS.map((c) => {
                const count = c === 'all' ? owned.length : owned.filter((o) => o.slot === c).length;
                return (
                  <button
                    key={c}
                    type="button"
                    aria-pressed={chip === c}
                    data-testid={`chip-${c}`}
                    onClick={() => setChip(c)}
                    className={cn(
                      'rx-focus-ring inline-flex h-8 items-center gap-1.5 rounded-full border px-3 text-xs font-medium transition-colors',
                      chip === c
                        ? 'border-accent bg-accent-soft text-accent-text'
                        : 'border-border bg-surface/60 text-fg-muted hover:border-border-strong/60 hover:text-fg',
                    )}
                  >
                    {t(`categories.${c}`)}
                    <span className="text-fg-subtle tabular-nums">{count}</span>
                  </button>
                );
              })}
            </div>

            {loading ? (
              <div
                className="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-3"
                aria-busy="true"
              >
                {Array.from({ length: 8 }, (_, i) => (
                  <Skeleton key={i} className="aspect-[4/5] rounded-xl" />
                ))}
              </div>
            ) : (
              <ul
                className="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-3"
                aria-label={t(`categories.${chip}`)}
                onPointerLeave={() => setHovered(null)}
              >
                {chip !== 'all' && (
                  <li>
                    <button
                      type="button"
                      data-testid="cosmetic-nothing"
                      aria-pressed={!current[chip]}
                      onClick={() =>
                        current[chip] &&
                        setLoadout(
                          unequip(current, chip),
                          t('toast.cleared', { slot: t(`categories.${chip}`) }),
                        )
                      }
                      onFocus={() => setHovered(null)}
                      onPointerEnter={() => setHovered(null)}
                      className={cn(
                        'rx-focus-ring flex aspect-[4/5] w-full flex-col items-center justify-center gap-2 rounded-xl border border-dashed text-sm transition-colors',
                        !current[chip]
                          ? 'border-accent text-accent-text'
                          : 'border-border text-fg-muted hover:border-border-strong hover:text-fg',
                      )}
                    >
                      <X className="size-6" aria-hidden />
                      {t('items.nothing')}
                    </button>
                  </li>
                )}
                {visible.map((c) => (
                  <li key={c.id}>
                    <CosmeticTile
                      cosmetic={c}
                      equipped={isEquipped(current, c)}
                      onSelect={() => onTile(c)}
                      onPreview={(on) => setHovered(on ? c : null)}
                    />
                  </li>
                ))}
              </ul>
            )}
            {!loading && chip !== 'all' && visible.length === 0 && (
              <EmptyState
                icon={<ShoppingBag />}
                title={t('empty.categoryTitle', { category: t(`categories.${chip}`) })}
                description={t('empty.categoryDescription')}
                action={
                  <Button size="sm" onClick={() => void invoke('app_open_url', { url: STORE_URL })}>
                    {t('empty.browse')}
                  </Button>
                }
              />
            )}

            <Outfits
              current={current}
              cosmetics={all}
              onWear={(l) => setLoadout(l, t('toast.outfitWorn'))}
            />

            <div className="rx-surface rounded-xl px-4 py-1">
              <SwitchRow
                label={t('switches.showMine')}
                description={t('switches.showMineHint')}
                checked={prefs.showMine}
                onCheckedChange={(v) => prefs.set({ showMine: v })}
              />
              <SwitchRow
                label={t('switches.hideOthers')}
                checked={prefs.hideOthers}
                onCheckedChange={(v) => prefs.set({ hideOthers: v })}
                className="border-border border-t"
              />
              <SwitchRow
                label={t('switches.hideTrailsAuras')}
                checked={prefs.hideTrailsAuras}
                onCheckedChange={(v) => prefs.set({ hideTrailsAuras: v })}
                className="border-border border-t"
              />
            </div>
          </section>
        </div>
      )}
    </Page>
  );
}

function CosmeticTile({
  cosmetic: c,
  equipped,
  onSelect,
  onPreview,
}: {
  cosmetic: Cosmetic;
  equipped: boolean;
  onSelect: () => void;
  onPreview: (on: boolean) => void;
}) {
  const { t } = useTranslation('locker');
  return (
    <button
      type="button"
      aria-pressed={equipped}
      data-testid={`cosmetic-${c.id}`}
      onClick={onSelect}
      onPointerEnter={() => onPreview(true)}
      onFocus={() => onPreview(true)}
      onBlur={() => onPreview(false)}
      title={c.description}
      className={cn(
        'rx-surface rx-focus-ring group relative flex w-full flex-col overflow-hidden rounded-xl text-left transition-[border-color,transform,box-shadow] duration-150 hover:-translate-y-0.5 hover:shadow-md',
        equipped ? 'border-accent shadow-glow' : 'border-border hover:border-border-strong/60',
      )}
    >
      <CosmeticArt
        slot={c.slot}
        color={c.visual.color}
        accent={c.visual.accent}
        className="aspect-square w-full"
      />
      <span className="flex flex-col gap-0.5 px-2.5 py-2">
        <span className="text-fg truncate text-sm font-medium">{c.name}</span>
        <span className={cn('text-2xs truncate font-medium', RARITY_CLASS[c.rarity])}>
          {t(`rarity.${c.rarity}`)} · {t(`categories.${c.slot}`)}
        </span>
      </span>
      {equipped && (
        <span className="bg-accent text-2xs text-accent-fg absolute top-2 right-2 inline-flex items-center gap-1 rounded-full px-2 py-0.5 font-semibold">
          <Check className="size-3" strokeWidth={3} aria-hidden />
          {t('items.equipped')}
        </span>
      )}
    </button>
  );
}

function Outfits({
  current,
  cosmetics,
  onWear,
}: {
  current: Loadout;
  cosmetics: Cosmetic[];
  onWear: (l: Loadout) => void;
}) {
  const { t } = useTranslation('locker');
  const { data, isLoading } = useOutfits();
  const { save, remove } = useOutfitActions();
  const [name, setName] = useState('');
  const outfits = data ?? [];
  const full = outfits.length >= MAX_OUTFITS;

  return (
    <section
      aria-labelledby="outfits-title"
      className="rx-surface flex flex-col gap-3 rounded-xl p-4"
    >
      <h2 id="outfits-title" className="text-sm font-semibold">
        {t('outfits.title')}{' '}
        <span className="text-fg-subtle font-normal">
          {t('outfits.count', { count: outfits.length, max: MAX_OUTFITS })}
        </span>
      </h2>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          const n = name.trim();
          if (!n || full) return;
          save.mutate(
            { name: n, loadout: current },
            {
              onSuccess: () => {
                toast({
                  tone: 'success',
                  title: t('toast.outfitSaved', { name: n }),
                  duration: 2500,
                });
                setName('');
              },
            },
          );
        }}
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('outfits.namePlaceholder')}
          aria-label={t('outfits.namePlaceholder')}
          maxLength={32}
          disabled={full}
          className="flex-1"
        />
        <Button
          type="submit"
          leftIcon={<Save />}
          loading={save.isPending}
          disabled={!name.trim() || full}
        >
          {t('outfits.save')}
        </Button>
      </form>
      {full && (
        <p className="text-warning-text text-xs">{t('outfits.full', { max: MAX_OUTFITS })}</p>
      )}
      {isLoading ? (
        <Skeleton className="h-10" />
      ) : outfits.length === 0 ? (
        <p className="text-fg-muted text-sm">{t('outfits.empty')}</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {outfits.map((o) => {
            const wearable = ownedOnly(o.loadout, cosmetics);
            const worn = sameLoadout(wearable, ownedOnly(current, cosmetics));
            return (
              <li
                key={o.id}
                className="hover:bg-surface-2 flex items-center gap-3 rounded-lg px-2 py-1.5"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm font-medium">{o.name}</span>
                  <span className="text-fg-subtle text-xs">
                    {t('outfits.items', { count: equippedCount(wearable) })}
                  </span>
                </span>
                {worn ? (
                  <Badge variant="accent" icon={<Check />}>
                    {t('outfits.wearing')}
                  </Badge>
                ) : (
                  <Button size="sm" variant="soft" onClick={() => onWear(wearable)}>
                    {t('outfits.wear')}
                  </Button>
                )}
                <IconButton
                  size="sm"
                  variant="danger"
                  label={t('outfits.delete', { name: o.name })}
                  icon={<Trash2 />}
                  onClick={() => remove.mutate(o.id)}
                />
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
