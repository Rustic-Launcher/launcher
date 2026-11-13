import {
  Badge,
  Button,
  cn,
  Field,
  Input,
  Select,
  Sheet,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  toast,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import { Archive, ChevronRight, FileArchive, Folder, PackageSearch } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useHorizonVersionLabel } from '@/components/HorizonVersion';
import { InstanceIcon, PRESET_COLORS, PRESET_ICONS } from '@/components/InstanceIcon';
import { invoke } from '@/ipc';
import type { InstanceIconPreset } from '@/ipc/types/instances';
import { formatDate } from '@/lib/format';
import { useHorizonVersions } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { useInstanceMutations } from './queries';

const PRESETS = Object.keys(PRESET_ICONS) as InstanceIconPreset[];

export function NewInstanceSheet({
  open,
  onOpenChange,
  initialTab = 'create',
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** The tab shown when the sheet opens (the Profiles page's "Import" opens on Import). */
  initialTab?: 'create' | 'import';
}) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const [tab, setTab] = useState<'create' | 'import'>(initialTab);
  useEffect(() => {
    if (open) setTab(initialTab);
  }, [open, initialTab]);
  return (
    <Sheet
      open={open}
      onOpenChange={onOpenChange}
      title={t('new.title')}
      description={t('new.description')}
      closeLabel={tc('actions.close')}
      width={480}
    >
      <Tabs variant="pill" value={tab} onValueChange={(v) => setTab(v as 'create' | 'import')}>
        <TabsList>
          <TabsTrigger value="create" data-testid="new-tab-create">
            {t('new.tabs.create')}
          </TabsTrigger>
          <TabsTrigger value="import" data-testid="new-tab-import">
            {t('new.tabs.import')}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="create">
          <CreateForm onDone={() => onOpenChange(false)} />
        </TabsContent>
        <TabsContent value="import">
          <ImportOptions onDone={() => onOpenChange(false)} />
        </TabsContent>
      </Tabs>
    </Sheet>
  );
}

function CreateForm({ onDone }: { onDone: () => void }) {
  const { t, i18n } = useTranslation('instances');
  const navigate = useNavigate();
  const versions = useHorizonVersions();
  const versionLabel = useHorizonVersionLabel();
  const setActive = usePrefs((s) => s.setActiveInstance);
  const { create } = useInstanceMutations();
  const [name, setName] = useState('');
  const [preset, setPreset] = useState<InstanceIconPreset>('grass');
  const [color, setColor] = useState(PRESET_COLORS[0]!);
  const [version, setVersion] = useState<string>('');
  const [touched, setTouched] = useState(false);

  useEffect(() => {
    if (!version && versions.data) {
      setVersion((versions.data.find((v) => v.recommended) ?? versions.data[0])?.id ?? '');
    }
  }, [versions.data, version]);

  const options = useMemo(
    () =>
      (versions.data ?? []).map((v) => ({
        value: v.id,
        label: (
          <span className="flex items-center gap-2">
            {versionLabel(v.id, v.rustic_edition)}
            {v.recommended && <Badge variant="accent">{t('new.recommended')}</Badge>}
          </span>
        ),
        description: [
          v.channel === 'nightly' ? t('new.nightly') : t('new.release'),
          formatDate(v.released_at, i18n.language),
          v.installed ? t('new.installed') : null,
        ]
          .filter(Boolean)
          .join(' · '),
      })),
    [versions.data, versionLabel, t, i18n.language],
  );

  const nameError = touched && !name.trim() ? t('new.nameRequired') : undefined;

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(e) => {
        e.preventDefault();
        setTouched(true);
        if (!name.trim() || !version) return;
        create.mutate(
          { name: name.trim(), icon: { kind: 'preset', preset, color }, horizon_version: version },
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
      <div className="flex items-center gap-4">
        <InstanceIcon icon={{ kind: 'preset', preset, color }} size="lg" />
        <Field label={t('new.name')} error={nameError} className="flex-1">
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
            />
          )}
        </Field>
      </div>

      <fieldset className="flex flex-col gap-2">
        <legend className="mb-2 text-sm font-medium">{t('new.icon')}</legend>
        <div role="radiogroup" aria-label={t('new.icon')} className="grid grid-cols-5 gap-2">
          {PRESETS.map((p) => {
            const Icon = PRESET_ICONS[p];
            const checked = preset === p;
            return (
              <button
                key={p}
                type="button"
                role="radio"
                aria-checked={checked}
                aria-label={t(`icons.${p}`)}
                onClick={() => setPreset(p)}
                className={cn(
                  'rx-focus-ring grid h-11 place-items-center rounded-lg border transition-colors',
                  checked
                    ? 'border-accent bg-accent-soft text-accent-text'
                    : 'border-border text-fg-muted hover:bg-surface-2',
                )}
              >
                <Icon className="size-5" />
              </button>
            );
          })}
        </div>
        <div role="radiogroup" aria-label={t('new.color')} className="mt-1 flex flex-wrap gap-2">
          {PRESET_COLORS.map((c, i) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={color === c}
              aria-label={t('new.colorN', { n: i + 1 })}
              onClick={() => setColor(c)}
              className={cn(
                'rx-focus-ring ring-offset-surface size-7 rounded-full ring-offset-2 transition-transform hover:scale-110',
                color === c && 'ring-fg ring-2',
              )}
              style={{ background: c }}
            />
          ))}
        </div>
      </fieldset>

      <Field label={t('new.version')} hint={t('new.versionHint')}>
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
            />
          )
        }
      </Field>

      <Button
        type="submit"
        variant="primary"
        size="lg"
        loading={create.isPending}
        data-testid="create-instance-submit"
      >
        {t('new.submit')}
      </Button>
    </form>
  );
}

function ImportOptions({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation('instances');
  const { importInstance } = useInstanceMutations();
  const [busy, setBusy] = useState<string | null>(null);

  const pick = async (kind: ImportKind) => {
    setBusy(kind);
    try {
      const path = await invoke('app_pick_path', {
        kind: kind === 'folder' ? 'folder' : 'file',
        title: t(`import.${kind}.title`),
        filters:
          kind === 'mrpack'
            ? [{ name: 'Modrinth', extensions: ['mrpack'] }]
            : kind === 'curseforge_zip'
              ? [{ name: 'CurseForge', extensions: ['zip'] }]
              : undefined,
      });
      if (!path) return;
      const res = await importInstance.mutateAsync({ kind, path });
      toast({
        tone: 'success',
        title: t('import.started', { name: res.instance.name }),
        description: t('import.background'),
      });
      onDone();
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <ImportOption
        kind="folder"
        icon={<Folder />}
        disabled={busy !== null}
        onPick={(k) => void pick(k)}
      />
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
      <Link
        to="/content"
        search={{ kind: 'modpack' }}
        onClick={onDone}
        className="rx-focus-ring border-border hover:bg-surface-2 flex items-center gap-3 rounded-xl border border-dashed p-4 text-left"
      >
        <span className="bg-surface-3 text-fg grid size-10 shrink-0 place-items-center rounded-lg [&_svg]:size-5">
          <PackageSearch />
        </span>
        <span className="flex flex-1 flex-col">
          <span className="text-sm font-semibold">{t('import.modpack.title')}</span>
          <span className="text-fg-muted text-xs">{t('import.modpack.description')}</span>
        </span>
        <ChevronRight className="text-fg-subtle size-4" />
      </Link>
      <p className="text-fg-subtle text-xs">{t('import.note')}</p>
    </div>
  );
}

type ImportKind = 'folder' | 'mrpack' | 'curseforge_zip';

function ImportOption({
  kind,
  icon,
  disabled,
  onPick,
}: {
  kind: ImportKind;
  icon: React.ReactNode;
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
      className="rx-focus-ring border-border bg-surface-2/60 hover:border-border-strong/60 hover:bg-surface-2 flex items-center gap-3 rounded-xl border p-4 text-left transition-colors disabled:opacity-50"
    >
      <span className="bg-accent-soft text-accent-text grid size-10 shrink-0 place-items-center rounded-lg [&_svg]:size-5">
        {icon}
      </span>
      <span className="flex flex-1 flex-col">
        <span className="text-sm font-semibold">{t(`import.${kind}.title`)}</span>
        <span className="text-fg-muted text-xs">{t(`import.${kind}.description`)}</span>
      </span>
      <ChevronRight className="text-fg-subtle size-4" />
    </button>
  );
}
