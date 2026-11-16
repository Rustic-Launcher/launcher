/**
 * The Java Launchpad (ADR 0172): a launch hero for the selected Java profile (install + owner
 * gate + start, progress, stop, crash), the profiles grid, and on the right the game's log and
 * the profile's mods.
 */
import {
  Badge,
  Button,
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownSeparator,
  DropdownTrigger,
  EmptyState,
  IconButton,
  Skeleton,
  toast,
  Tooltip,
} from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { motion } from 'motion/react';
import {
  AlertTriangle,
  Check,
  ChevronDown,
  FolderOpen,
  Play,
  Plus,
  Settings2,
  Square,
  WifiOff,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import type { JavaProfile, JavaRunState } from '@/ipc/types/java';
import { formatRelative } from '@/lib/format';
import { usePrefs } from '@/stores/prefs';
import { EditionSwitcher } from './EditionSwitcher';
import { EditionLogo } from './EditionLogo';
import { JavaAside } from './JavaAside';
import {
  errorKey,
  formatMemory,
  formatPlayTime,
  launchAction,
  LOADER_NAMES,
  pickProfile,
  profileVersionLabel,
  stepKey,
} from './logic';
import { ProfileSheet } from './ProfileSheet';
import { jk, useJavaEvents, useJavaLive, useJavaProfiles } from './queries';

export function JavaLaunchpad() {
  const { t } = useTranslation('java');
  const { data: profiles, isLoading } = useJavaProfiles();
  const remembered = usePrefs((s) => s.javaProfileId);
  const select = usePrefs((s) => s.setJavaProfileId);
  const profile = pickProfile(profiles, remembered);
  const [sheet, setSheet] = useState<{ mode: 'create' } | { mode: 'edit'; id: string } | null>(
    null,
  );
  useJavaEvents(profile?.id);
  const editing = sheet?.mode === 'edit' ? profiles?.find((p) => p.id === sheet.id) : undefined;

  return (
    <>
      <div className="flex min-w-0 flex-col gap-5" data-testid="java-launchpad">
        <JavaHero
          profile={profile}
          profiles={profiles ?? []}
          loading={isLoading}
          onSelect={select}
          onCreate={() => setSheet({ mode: 'create' })}
          onEdit={(id) => setSheet({ mode: 'edit', id })}
        />
        <section aria-labelledby="java-profiles" className="flex flex-col gap-3">
          <div className="flex items-center justify-between px-1">
            <h2 id="java-profiles" className="text-base font-semibold">
              {t('profiles.title')}
            </h2>
            <Button
              size="sm"
              variant="soft"
              leftIcon={<Plus />}
              onClick={() => setSheet({ mode: 'create' })}
              data-testid="java-new-profile"
            >
              {t('profiles.new')}
            </Button>
          </div>
          {isLoading && (
            <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
              {Array.from({ length: 2 }, (_, i) => (
                <Skeleton key={i} className="h-28 rounded-xl" />
              ))}
            </div>
          )}
          {profiles && !profiles.length && (
            <EmptyState
              icon={<EditionLogo edition="java" size={28} />}
              title={t('profiles.empty')}
              description={t('profiles.emptyHint')}
              action={
                <Button variant="primary" onClick={() => setSheet({ mode: 'create' })}>
                  {t('profiles.new')}
                </Button>
              }
            />
          )}
          <ul className="grid grid-cols-1 gap-3 lg:grid-cols-2" data-testid="java-profiles">
            {profiles?.map((p) => (
              <ProfileCard
                key={p.id}
                profile={p}
                selected={p.id === profile?.id}
                onSelect={() => select(p.id)}
                onEdit={() => setSheet({ mode: 'edit', id: p.id })}
              />
            ))}
          </ul>
        </section>
      </div>
      <JavaAside profile={profile} onEdit={(id) => setSheet({ mode: 'edit', id })} />
      <ProfileSheet
        open={!!sheet}
        onOpenChange={(o) => !o && setSheet(null)}
        profile={editing}
        onCreated={(p) => {
          select(p.id);
          setSheet(null);
        }}
        onDeleted={() => {
          select(null);
          setSheet(null);
        }}
      />
    </>
  );
}

/** Launch, install, stop or cancel, with the owner gate's and the install's messages. */
export function useJavaLaunch(profile: JavaProfile | undefined) {
  const { t } = useTranslation('java');
  const qc = useQueryClient();
  const state = useJavaLive((s) => (profile ? s.states[profile.id] : undefined));
  const [pending, setPending] = useState(false);
  const action = profile ? launchAction(profile, state) : 'launch';
  const run = async () => {
    if (!profile) return;
    setPending(true);
    try {
      if (action === 'stop' || action === 'cancel') {
        await invoke('java_profile_stop', { id: profile.id });
      } else if (action === 'launch' || action === 'install_launch') {
        const s = await invoke('java_profile_launch', { id: profile.id });
        useJavaLive.getState().setState(s);
        if (s.offline) toast({ tone: 'warning', title: t('hero.offline') });
      }
    } catch (e) {
      const f = e instanceof IpcFailure ? e : null;
      const reason = f?.details?.reason as string | undefined;
      const key = errorKey(reason);
      toast({
        tone: 'error',
        title: t('error.title'),
        description: key ? t(key) : (f?.message ?? String(e)),
      });
    } finally {
      setPending(false);
      void qc.invalidateQueries({ queryKey: jk.profiles });
    }
  };
  return { state, action, pending, run };
}

function JavaHero({
  profile,
  profiles,
  loading,
  onSelect,
  onCreate,
  onEdit,
}: {
  profile: JavaProfile | undefined;
  profiles: JavaProfile[];
  loading: boolean;
  onSelect: (id: string) => void;
  onCreate: () => void;
  onEdit: (id: string) => void;
}) {
  const { t, i18n } = useTranslation('java');
  const { state, action, pending, run } = useJavaLaunch(profile);
  const progress = state?.state === 'installing' ? (state.progress ?? 0) : null;
  const line1 =
    action === 'cancel'
      ? t(stepKey(state?.step))
      : action === 'starting'
        ? t('hero.starting')
        : action === 'stop'
          ? t('hero.stop')
          : action === 'install_launch'
            ? t('hero.installLaunch')
            : t('hero.launch');
  const line2 = profile ? profileVersionLabel(profile) : t('hero.noProfile');
  const busy = action === 'cancel' || action === 'starting';
  const err = state?.state === 'idle' && state.message ? state : undefined;

  return (
    <section
      aria-label={t('hero.label')}
      data-testid="java-hero"
      className="border-border-default relative isolate flex aspect-[16/7] min-h-72 flex-col items-center justify-center gap-4 overflow-hidden rounded-2xl border px-4"
      style={{
        background:
          'radial-gradient(120% 90% at 20% 10%, rgba(76,140,64,0.45), transparent 60%), radial-gradient(90% 80% at 85% 90%, rgba(120,86,52,0.5), transparent 65%), linear-gradient(160deg, #1b2a1d 0%, #141a17 55%, #15120f 100%)',
      }}
    >
      <JavaHeroArt />
      <EditionSwitcher overlay />
      <div className="relative flex items-center gap-2 text-sm font-semibold text-white/85">
        <EditionLogo edition="java" size={22} />
        {t('switcher.java')}
      </div>
      {loading ? (
        <Skeleton className="h-[72px] w-80 rounded-xl" />
      ) : (
        <div
          className={cn(
            'shadow-glow relative flex h-[72px] w-[min(360px,100%)] overflow-hidden rounded-xl',
            action === 'stop' || busy ? 'bg-surface-3 shadow-none' : 'bg-accent',
          )}
        >
          {progress != null && (
            <motion.span
              aria-hidden
              className="bg-accent absolute inset-y-0 start-0"
              initial={false}
              animate={{ width: `${progress * 100}%` }}
              transition={{ duration: 0.25 }}
            />
          )}
          <button
            type="button"
            data-testid="java-play"
            data-action={action}
            disabled={!profile || pending || action === 'starting'}
            onClick={() => void run()}
            aria-label={action === 'cancel' ? t('hero.cancel') : line1}
            className={cn(
              'rx-focus-ring group relative flex flex-1 flex-col items-center justify-center gap-0.5 px-4 transition-colors disabled:opacity-60',
              action === 'stop' || busy ? 'text-fg hover:bg-surface-2/40' : 'text-accent-fg',
            )}
          >
            <span className="flex items-center gap-2 text-lg font-black tracking-wide uppercase">
              {action === 'stop' ? (
                <Square className="size-4 fill-current" />
              ) : action === 'cancel' ? null : (
                <Play className="size-4 fill-current" />
              )}
              <span className={cn(action === 'cancel' && 'group-hover:hidden')}>{line1}</span>
              {action === 'cancel' && (
                <span className="hidden group-hover:inline">{t('hero.cancel')}</span>
              )}
            </span>
            <span className="text-xs font-semibold opacity-80" data-testid="java-play-version">
              {progress != null ? `${Math.round(progress * 100)} % · ${line2}` : line2}
            </span>
          </button>
          <Dropdown>
            <DropdownTrigger asChild>
              <button
                type="button"
                data-testid="java-profile-selector"
                aria-label={t('hero.pickProfile')}
                className={cn(
                  'rx-focus-ring relative grid w-12 place-items-center border-s border-black/15',
                  action === 'stop' || busy ? 'text-fg' : 'text-accent-fg',
                )}
              >
                <ChevronDown className="size-5" />
              </button>
            </DropdownTrigger>
            <DropdownContent align="end" className="w-72">
              <DropdownLabel>{t('profiles.title')}</DropdownLabel>
              {profiles.map((p) => (
                <DropdownItem key={p.id} onSelect={() => onSelect(p.id)}>
                  <span className="flex w-full items-center justify-between gap-2">
                    <span className="flex min-w-0 flex-col">
                      <span className="truncate">{p.name}</span>
                      <span className="text-fg-subtle truncate text-xs">
                        {profileVersionLabel(p)}
                      </span>
                    </span>
                    {p.id === profile?.id && <Check className="text-accent-text size-4" />}
                  </span>
                </DropdownItem>
              ))}
              <DropdownSeparator />
              <DropdownItem icon={<Plus />} onSelect={onCreate}>
                {t('profiles.new')}
              </DropdownItem>
            </DropdownContent>
          </Dropdown>
        </div>
      )}
      <div className="relative flex min-h-6 flex-wrap items-center justify-center gap-2 text-xs">
        {profile && state?.state === 'running' && (
          <span className="bg-surface/70 text-fg flex items-center gap-1.5 rounded-full px-2.5 py-1 backdrop-blur">
            <span className="bg-success size-1.5 rounded-full" />
            <span data-testid="java-status">{t('hero.running')}</span>
          </span>
        )}
        {state?.offline && state.state === 'running' && (
          <Badge variant="warning" icon={<WifiOff />}>
            {t('hero.offlineShort')}
          </Badge>
        )}
        {profile && !state?.state && profile.last_played_at && (
          <span className="text-white/75" data-testid="java-status">
            {t('hero.lastPlayed', {
              when: formatRelative(
                new Date(profile.last_played_at * 1000).toISOString(),
                i18n.language,
              ),
            })}
          </span>
        )}
        {state?.state === 'crashed' && (
          <span
            role="alert"
            className="bg-danger/85 flex items-center gap-1.5 rounded-full px-2.5 py-1 font-medium text-white"
            data-testid="java-crashed"
          >
            <AlertTriangle className="size-3.5" />
            {t('hero.crashed', { code: state.exit_code ?? '?' })}
          </span>
        )}
        {err && (
          <span
            role="alert"
            className="bg-surface/80 text-fg max-w-[34rem] rounded-lg px-3 py-1.5 text-center backdrop-blur"
            data-testid="java-error"
          >
            {errorKey(err.reason) ? t(errorKey(err.reason)!) : err.message}
          </span>
        )}
      </div>
      {profile && (
        <div className="absolute end-3 bottom-3 flex gap-2">
          <Tooltip content={t('hero.openFolder')}>
            <IconButton
              size="sm"
              label={t('hero.openFolder')}
              icon={<FolderOpen />}
              className="bg-surface/80 backdrop-blur"
              onClick={() =>
                void invoke('java_profile_open_folder', { id: profile.id, folder: 'game' })
              }
            />
          </Tooltip>
          <Tooltip content={t('hero.settings')}>
            <IconButton
              size="sm"
              label={t('hero.settings')}
              icon={<Settings2 />}
              className="bg-surface/80 backdrop-blur"
              data-testid="java-hero-settings"
              onClick={() => onEdit(profile.id)}
            />
          </Tooltip>
        </div>
      )}
    </section>
  );
}

/** Original blocky art drawn in code (no game asset). */
function JavaHeroArt() {
  const cells = Array.from({ length: 40 }, (_, i) => {
    const x = (i % 10) * 10;
    const y = Math.floor(i / 10) * 10;
    const v = (i * 37 + 11) % 100;
    return { x, y, o: v > 75 ? 0.09 : v > 45 ? 0.045 : 0.02 };
  });
  return (
    <svg
      aria-hidden
      viewBox="0 0 100 40"
      preserveAspectRatio="xMidYMax slice"
      className="absolute inset-x-0 bottom-0 h-1/2 w-full"
    >
      {cells.map((c, i) => (
        <rect key={i} x={c.x} y={c.y} width="9.4" height="9.4" fill="white" fillOpacity={c.o} />
      ))}
    </svg>
  );
}

function ProfileCard({
  profile: p,
  selected,
  onSelect,
  onEdit,
}: {
  profile: JavaProfile;
  selected: boolean;
  onSelect: () => void;
  onEdit: () => void;
}) {
  const { t } = useTranslation('java');
  const { state, action, pending, run } = useJavaLaunch(p);
  const played = formatPlayTime(p.play_time);
  return (
    <li>
      <div
        data-testid={`java-profile-${p.id}`}
        data-selected={selected}
        className={cn(
          'bg-surface hover:bg-surface-2 flex items-center gap-3 rounded-xl border p-3 transition-colors',
          selected ? 'border-accent/60' : 'border-border',
        )}
      >
        <button
          type="button"
          onClick={onSelect}
          className="rx-focus-ring flex min-w-0 flex-1 items-center gap-3 rounded-lg text-start"
        >
          <span className="bg-surface-3 grid size-12 shrink-0 place-items-center rounded-lg">
            <EditionLogo edition="java" size={26} />
          </span>
          <span className="flex min-w-0 flex-col gap-1">
            <span className="truncate font-semibold">{p.name}</span>
            <span className="text-fg-muted flex flex-wrap items-center gap-1.5 text-xs">
              <Badge size="sm" variant={p.loader === 'vanilla' ? 'neutral' : 'accent'}>
                {LOADER_NAMES[p.loader]}
              </Badge>
              <span>{p.game_version}</span>
              <span aria-hidden>·</span>
              <span>{formatMemory(p.memory_max_mb)}</span>
              {p.play_time > 0 && (
                <>
                  <span aria-hidden>·</span>
                  <span>
                    {'h' in played
                      ? t('profiles.hours', { n: played.h })
                      : t('profiles.minutes', { n: played.min })}
                  </span>
                </>
              )}
            </span>
            {!p.installed && (
              <span className="text-fg-subtle text-xs">{t('profiles.notInstalled')}</span>
            )}
            {state?.state === 'installing' && (
              <span className="text-accent-text text-xs">
                {t(stepKey(state.step))} · {Math.round((state.progress ?? 0) * 100)} %
              </span>
            )}
          </span>
        </button>
        <IconButton
          size="sm"
          label={t('hero.settings')}
          icon={<Settings2 />}
          onClick={onEdit}
          data-testid={`java-edit-${p.id}`}
        />
        <IconButton
          size="sm"
          variant={action === 'stop' ? 'secondary' : 'primary'}
          label={
            action === 'stop'
              ? t('hero.stop')
              : action === 'cancel'
                ? t('hero.cancel')
                : t('hero.launch')
          }
          icon={action === 'stop' || action === 'cancel' ? <Square /> : <Play />}
          disabled={pending || action === 'starting'}
          onClick={() => void run()}
          data-testid={`java-play-${p.id}`}
        />
      </div>
    </li>
  );
}

export type { JavaRunState };
