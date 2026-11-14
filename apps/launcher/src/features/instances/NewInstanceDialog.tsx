import { Badge, Button, cn, Dialog, Field, Input, Select, Skeleton, toast } from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  Archive,
  ChevronRight,
  Clock,
  FileArchive,
  FileUp,
  Folder,
  Layers,
  Moon,
  Package,
  PackageSearch,
  Sparkles,
  Sprout,
  Upload,
} from 'lucide-react';
import { RadioGroup } from 'radix-ui';
import { useEffect, useMemo, useState, type DragEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogTabList,
  CatalogTabPanel,
  CatalogTabs,
  MetaItem,
  OverlayBadge,
  VisualCard,
} from '@/components/catalog';
import { IconPicker } from '@/components/IconPicker';
import { InstanceIcon } from '@/components/InstanceIcon';
import { invoke, isNative } from '@/ipc';
import type {
  HorizonVersion,
  InstanceIcon as Icon,
  InstanceIconPreset,
} from '@/ipc/types/instances';
import { formatDate, shortVersion } from '@/lib/format';
import { useHorizonVersions } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { InstanceCover } from './InstanceCover';
import { importKindOf, type ImportKind } from './logic';
import { useInstanceMutations } from './queries';

type Tab = 'create' | 'import';
type Edition = 'rustic' | 'horizon';

/** The draft profile's cover seed: stable while typing, so the art doesn't jump. */
const DRAFT_ID = 'new-profile';

/**
 * "New instance" (`?new=1|import`, the rail's "+", the Profiles page's Create and Import): a modal
 * with a Create tab (name, icon, edition and build, beside a live preview of the profile's card as
 * the Profiles grid will show it) and an Import tab (a drop zone for a pack or an instance folder,
 * then the supported formats and modpacks from Explore).
 */
export function NewInstanceDialog({
  open,
  onOpenChange,
  initialTab = 'create',
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** The tab shown when the dialog opens (the Profiles page's "Import" opens on Import). */
  initialTab?: Tab;
}) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const [tab, setTab] = useState<Tab>(initialTab);
  useEffect(() => {
    if (open) setTab(initialTab);
  }, [open, initialTab]);
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('new.title')}
      description={t('new.description')}
      closeLabel={tc('actions.close')}
      size="lg"
      className="max-w-4xl"
    >
      <CatalogTabs value={tab} onValueChange={setTab}>
        <CatalogTabList
          label={t('new.title')}
          testIdPrefix="new-tab"
          tabs={[
            { value: 'create', label: t('new.tabs.create'), icon: <Sprout /> },
            { value: 'import', label: t('new.tabs.import'), icon: <Upload /> },
          ]}
        />
        <CatalogTabPanel value="create">
          <CreateForm onDone={() => onOpenChange(false)} />
        </CatalogTabPanel>
        <CatalogTabPanel value="import">
          <ImportPanel active={open && tab === 'import'} onDone={() => onOpenChange(false)} />
        </CatalogTabPanel>
      </CatalogTabs>
    </Dialog>
  );
}

/** The build the form starts on for an edition: its recommended one, else its newest release. */
function defaultBuild(list: HorizonVersion[]): string {
  return (
    (list.find((v) => v.recommended) ?? list.find((v) => v.channel === 'release') ?? list[0])?.id ??
    ''
  );
}

function CreateForm({ onDone }: { onDone: () => void }) {
  const { t, i18n } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const navigate = useNavigate();
  const versions = useHorizonVersions();
  const setActive = usePrefs((s) => s.setActiveInstance);
  const { create } = useInstanceMutations();
  const [name, setName] = useState('');
  const [preset, setPreset] = useState<InstanceIconPreset>('grass');
  const [color, setColor] = useState('#4E9F5B');
  const [edition, setEdition] = useState<Edition | null>(null);
  const [version, setVersion] = useState<string>('');
  const [touched, setTouched] = useState(false);

  const byEdition = useMemo(() => {
    const all = versions.data ?? [];
    return {
      rustic: all.filter((v) => v.rustic_edition),
      horizon: all.filter((v) => !v.rustic_edition),
    };
  }, [versions.data]);

  // Start on the recommended build's edition (the Rustic edition when there is one).
  useEffect(() => {
    if (edition || !versions.data) return;
    const rec = versions.data.find((v) => v.recommended);
    const start: Edition = rec
      ? rec.rustic_edition
        ? 'rustic'
        : 'horizon'
      : byEdition.rustic.length
        ? 'rustic'
        : 'horizon';
    setEdition(start);
    setVersion(defaultBuild(byEdition[start]));
  }, [versions.data, edition, byEdition]);

  const builds = edition ? byEdition[edition] : [];
  const selected = builds.find((v) => v.id === version);
  const icon: Icon = { kind: 'preset', preset, color };

  const options = builds.map((v) => ({
    value: v.id,
    label: (
      <span className="flex items-center gap-2">
        {tc('horizon.version', { version: shortVersion(v.id) })}
        {v.recommended && <Badge variant="accent">{t('new.recommended')}</Badge>}
        {v.channel === 'nightly' && <Badge variant="warning">{t('new.nightly')}</Badge>}
      </span>
    ),
    description: [
      v.channel === 'nightly' ? t('new.nightly') : t('new.release'),
      formatDate(v.released_at, i18n.language),
      v.installed ? t('new.installed') : null,
    ]
      .filter(Boolean)
      .join(' · '),
  }));

  const nameError = touched && !name.trim() ? t('new.nameRequired') : undefined;

  return (
    <form
      className="grid gap-6 md:grid-cols-[minmax(0,1fr)_16.5rem]"
      noValidate
      onSubmit={(e) => {
        e.preventDefault();
        setTouched(true);
        if (!name.trim() || !version) return;
        create.mutate(
          { name: name.trim(), icon, horizon_version: version },
          {
            onSuccess: (inst) => {
              setActive(inst.id);
              toast({ tone: 'success', title: t('new.created', { name: inst.name }) });
              onDone();
              void navigate({
                to: '/instances/$instanceId',
                params: { instanceId: inst.id },
                search: {},
              });
            },
          },
        );
      }}
    >
      <div className="flex min-w-0 flex-col gap-5">
        <Field label={t('new.name')} error={nameError}>
          {(p) => (
            <Input
              {...p}
              value={name}
              onChange={(e) => setName(e.target.value)}
              onBlur={() => setTouched(true)}
              placeholder={t('new.namePlaceholder')}
              maxLength={48}
              invalid={!!nameError}
              data-testid="new-instance-name"
              data-autofocus
            />
          )}
        </Field>

        <div className="flex flex-col gap-2">
          <span className="text-sm font-medium">{t('new.icon')}</span>
          <IconPicker
            preset={preset}
            color={color}
            onPreset={setPreset}
            onColor={setColor}
            iconsLabel={t('new.icon')}
            colorsLabel={t('new.color')}
            iconLabel={(p) => t(`icons.${p}`)}
            colorLabel={(n) => t('new.colorN', { n })}
          />
        </div>

        <div className="flex flex-col gap-2">
          <span id="new-edition" className="text-sm font-medium">
            {t('new.edition')}
          </span>
          {versions.isLoading ? (
            <Skeleton className="h-20 rounded-xl" />
          ) : (
            <RadioGroup.Root
              aria-labelledby="new-edition"
              value={edition ?? ''}
              onValueChange={(v) => {
                const e = v as Edition;
                setEdition(e);
                setVersion(defaultBuild(byEdition[e]));
              }}
              className="grid gap-2 sm:grid-cols-2"
            >
              <EditionCard
                value="rustic"
                checked={edition === 'rustic'}
                disabled={byEdition.rustic.length === 0}
                title={tc('horizon.rusticEdition')}
                badge={<Badge variant="accent">{t('new.recommended')}</Badge>}
                description={byEdition.rustic.length ? t('new.editionRustic') : t('new.noBuild')}
              />
              <EditionCard
                value="horizon"
                checked={edition === 'horizon'}
                disabled={byEdition.horizon.length === 0}
                title={t('profiles.filters.editions.horizon')}
                description={byEdition.horizon.length ? t('new.editionHorizon') : t('new.noBuild')}
              />
            </RadioGroup.Root>
          )}
        </div>

        <Field
          label={t('new.version')}
          hint={selected?.channel === 'nightly' ? t('new.nightlyHint') : t('new.versionHint')}
        >
          {(p) =>
            versions.isLoading ? (
              <Skeleton className="h-9" />
            ) : (
              <Select
                id={p.id}
                value={version}
                onValueChange={setVersion}
                options={options}
                aria-label={t('new.version')}
                disabled={builds.length === 0}
              />
            )
          }
        </Field>
      </div>

      <div className="flex flex-col gap-3 md:sticky md:top-0 md:self-start">
        <p className="rx-eyebrow text-fg-subtle px-1">{t('new.preview')}</p>
        <ProfilePreview
          name={name.trim()}
          icon={icon}
          build={selected}
          placeholder={t('new.namePlaceholder')}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          loading={create.isPending}
          data-testid="create-instance-submit"
          className="w-full"
        >
          {t('new.submit')}
        </Button>
      </div>
    </form>
  );
}

function EditionCard({
  value,
  checked,
  disabled,
  title,
  badge,
  description,
}: {
  value: Edition;
  checked: boolean;
  disabled: boolean;
  title: string;
  badge?: ReactNode;
  description: string;
}) {
  return (
    <RadioGroup.Item
      value={value}
      disabled={disabled}
      data-testid={`new-edition-${value}`}
      className={cn(
        'rx-focus-ring flex flex-col items-start gap-1 rounded-xl border p-3 text-left transition-[border-color,background-color] duration-150',
        checked
          ? 'border-accent bg-accent-soft'
          : 'border-border bg-surface-2/50 hover:border-border-strong/70',
        disabled && 'hover:border-border cursor-not-allowed opacity-55',
      )}
    >
      <span className="text-fg flex flex-wrap items-center gap-2 text-sm font-semibold">
        {title}
        {badge}
      </span>
      <span className="text-fg-muted text-xs leading-relaxed">{description}</span>
    </RadioGroup.Item>
  );
}

/** The card the new profile will get on the Profiles grid, drawn from the form as it changes. */
function ProfilePreview({
  name,
  icon,
  build,
  placeholder,
}: {
  name: string;
  icon: Icon;
  build: HorizonVersion | undefined;
  placeholder: string;
}) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  return (
    <VisualCard
      seed={DRAFT_ID}
      title={name || placeholder}
      testId="new-instance-preview"
      className="hover:translate-y-0"
      coverArt={<InstanceCover instance={{ id: DRAFT_ID, icon }} />}
      thumb={
        <InstanceIcon icon={icon} size="md" className="ring-surface relative -mt-7 mb-0.5 ring-4" />
      }
      renderTitle={(cls) => (
        <span className={cn(cls, 'hover:text-fg', !name && 'text-fg-subtle')}>
          {name || placeholder}
        </span>
      )}
      overlay={
        build?.channel === 'nightly' ? (
          <OverlayBadge tone="warning" icon={<Moon />}>
            {t('new.nightly')}
          </OverlayBadge>
        ) : undefined
      }
      coverFooter={
        build ? (
          <div className="flex items-center justify-end gap-1.5 px-2.5 pb-2.5">
            <OverlayBadge>
              {tc('horizon.version', { version: shortVersion(build.id) })}
            </OverlayBadge>
            {build.rustic_edition && (
              <OverlayBadge tone="accent">{tc('horizon.rusticEdition')}</OverlayBadge>
            )}
          </div>
        ) : undefined
      }
      byline={
        <span className="flex items-center gap-1">
          <Clock className="size-3 shrink-0" aria-hidden />
          {t('profiles.card.neverPlayed')}
        </span>
      }
      meta={
        <>
          <MetaItem icon={<Package />} label={t('profiles.card.mods')}>
            0
          </MetaItem>
          <MetaItem icon={<Layers />} label={t('profiles.card.packs')}>
            0
          </MetaItem>
          <MetaItem icon={<Sparkles />} label={t('profiles.card.shaders')}>
            0
          </MetaItem>
        </>
      }
    />
  );
}

function ImportPanel({ active, onDone }: { active: boolean; onDone: () => void }) {
  const { t } = useTranslation('instances');
  const { importInstance } = useInstanceMutations();
  const [busy, setBusy] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  const run = async (kind: ImportKind, path: string) => {
    setBusy(kind);
    try {
      const res = await importInstance.mutateAsync({ kind, path });
      toast({
        tone: 'success',
        title: t('import.started', { name: res.instance.name }),
        description: t('import.background'),
      });
      onDone();
    } catch {
      // The mutation's own error toast says why.
    } finally {
      setBusy(null);
    }
  };

  const fromPath = (path: string) => {
    const kind = importKindOf(path);
    if (!kind) {
      toast({ tone: 'error', title: t('import.drop.unsupported') });
      return;
    }
    void run(kind, path);
  };

  const pick = async (kind: ImportKind | 'any') => {
    const path = await invoke('app_pick_path', {
      kind: kind === 'folder' ? 'folder' : 'file',
      title: kind === 'any' ? t('import.drop.choose') : t(`import.${kind}.title`),
      filters:
        kind === 'mrpack'
          ? [{ name: 'Modrinth', extensions: ['mrpack'] }]
          : kind === 'curseforge_zip'
            ? [{ name: 'CurseForge', extensions: ['zip'] }]
            : kind === 'any'
              ? [{ name: 'Modrinth, CurseForge', extensions: ['mrpack', 'zip'] }]
              : undefined,
    });
    if (!path) return;
    if (kind === 'any') fromPath(path);
    else void run(kind, path);
  };

  // In the desktop app, files dropped on the window arrive as paths through the webview's
  // drag-and-drop events (the page's own drop events don't carry them).
  useEffect(() => {
    if (!active || !isNative()) return;
    let un: (() => void) | undefined;
    let cancelled = false;
    void import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent((e) => {
          const p = e.payload;
          if (p.type === 'enter' || p.type === 'over') setDragging(true);
          else if (p.type === 'leave') setDragging(false);
          else if (p.type === 'drop') {
            setDragging(false);
            const first = p.paths[0];
            if (first) fromPath(first);
          }
        })
        .then((u) => {
          if (cancelled) u();
          else un = u;
        }),
    );
    return () => {
      cancelled = true;
      un?.();
    };
    // fromPath is recreated each render; the subscription only follows `active`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  // The browser build (mock): the page's own drop event, with the file name standing for a path.
  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setDragging(false);
    if (isNative()) return;
    const file = e.dataTransfer.files[0];
    if (file) fromPath(file.name);
  };

  return (
    <div className="grid gap-5 md:grid-cols-[minmax(0,1fr)_18rem]">
      <div
        data-testid="import-drop"
        data-dragging={dragging || undefined}
        onDragEnter={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragOver={(e) => e.preventDefault()}
        onDragLeave={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDragging(false);
        }}
        onDrop={onDrop}
        className={cn(
          'flex min-h-64 flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed px-6 py-8 text-center transition-colors duration-150',
          dragging ? 'border-accent bg-accent-soft/60' : 'border-border-strong/50 bg-surface-2/40',
        )}
      >
        <span
          aria-hidden
          className={cn(
            'grid size-14 place-items-center rounded-2xl transition-colors [&_svg]:size-7',
            dragging ? 'bg-accent text-accent-fg' : 'bg-surface-3 text-fg',
          )}
        >
          <FileUp />
        </span>
        <p className="text-fg text-base font-semibold" aria-live="polite">
          {dragging ? t('import.drop.active') : t('import.drop.title')}
        </p>
        <p className="text-fg-muted max-w-sm text-sm">{t('import.drop.hint')}</p>
        <Button
          variant="primary"
          leftIcon={<FileArchive />}
          loading={busy !== null}
          onClick={() => void pick('any')}
          data-testid="import-choose"
        >
          {t('import.drop.choose')}
        </Button>
      </div>

      <div className="flex flex-col gap-2">
        <p className="text-fg-subtle px-1 text-xs font-semibold tracking-wider uppercase">
          {t('import.formats')}
        </p>
        <ImportOption
          kind="mrpack"
          icon={<FileArchive />}
          disabled={busy !== null}
          onPick={(k) => void pick(k)}
        />
        <ImportOption
          kind="curseforge_zip"
          icon={<Archive />}
          disabled={busy !== null}
          onPick={(k) => void pick(k)}
        />
        <ImportOption
          kind="folder"
          icon={<Folder />}
          disabled={busy !== null}
          onPick={(k) => void pick(k)}
        />
        <Link
          to="/content"
          search={{ kind: 'modpack' }}
          onClick={onDone}
          className="rx-focus-ring border-border hover:bg-surface-2 flex items-center gap-3 rounded-xl border border-dashed p-3 text-left"
        >
          <span className="bg-surface-3 text-fg grid size-9 shrink-0 place-items-center rounded-lg [&_svg]:size-4">
            <PackageSearch />
          </span>
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="text-sm font-semibold">{t('import.modpack.title')}</span>
            <span className="text-fg-muted text-xs">{t('import.modpack.description')}</span>
          </span>
          <ChevronRight className="text-fg-subtle size-4 shrink-0" />
        </Link>
        <p className="text-fg-subtle px-1 pt-1 text-xs">{t('import.note')}</p>
      </div>
    </div>
  );
}

function ImportOption({
  kind,
  icon,
  disabled,
  onPick,
}: {
  kind: ImportKind;
  icon: ReactNode;
  disabled: boolean;
  onPick: (k: ImportKind) => void;
}) {
  const { t } = useTranslation('instances');
  return (
    <button
      type="button"
      onClick={() => onPick(kind)}
      disabled={disabled}
      data-testid={`import-${kind}`}
      className="rx-focus-ring border-border bg-surface-2/60 hover:border-border-strong/60 hover:bg-surface-2 flex items-center gap-3 rounded-xl border p-3 text-left transition-colors disabled:opacity-50"
    >
      <span className="bg-accent-soft text-accent-text grid size-9 shrink-0 place-items-center rounded-lg [&_svg]:size-4">
        {icon}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="text-sm font-semibold">{t(`import.${kind}.title`)}</span>
        <span className="text-fg-muted text-xs">{t(`import.${kind}.description`)}</span>
      </span>
      <ChevronRight className="text-fg-subtle size-4 shrink-0" />
    </button>
  );
}
