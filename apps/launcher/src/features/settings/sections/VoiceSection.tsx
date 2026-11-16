/**
 * Settings › Voice (ADR 0176): devices, voice activity or push-to-talk, sensitivity with a live
 * meter, processing, output volume, the in-game overlay, servers' proximity voice, and the
 * Rustic Servers this account turned proximity voice on for (their plugin keys).
 */
import { Button, Dialog, IconButton, Input, Select, Slider, Switch, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Copy, KeyRound, Mic, Trash2 } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import { errorCode } from '@/cloud/errors';
import type { VoiceServerInfo } from '@/cloud/types';
import { keyLabel, mouseCode } from '@/features/voice/keys';
import { micConstraints, useVoiceSettings } from '@/features/voice/settings';
import { meterValue, rmsDb, thresholdDb } from '@/features/voice/vad';
import { useMe } from '@/queries/social';
import { SettingRow, SettingsCard } from '../components';

function useDevices() {
  const [devices, setDevices] = useState<MediaDeviceInfo[]>([]);
  useEffect(() => {
    const md = navigator.mediaDevices;
    if (!md?.enumerateDevices) return;
    const refresh = () =>
      void md
        .enumerateDevices()
        .then(setDevices)
        .catch(() => undefined);
    refresh();
    md.addEventListener?.('devicechange', refresh);
    return () => md.removeEventListener?.('devicechange', refresh);
  }, []);
  return {
    inputs: devices.filter((d) => d.kind === 'audioinput'),
    outputs: devices.filter((d) => d.kind === 'audiooutput'),
    refresh: () => void navigator.mediaDevices?.enumerateDevices().then(setDevices),
  };
}

/** The microphone test: a live level meter against the sensitivity's threshold. */
function MicTest({ onDevices }: { onDevices: () => void }) {
  const { t } = useTranslation('settings');
  const settings = useVoiceSettings();
  const [level, setLevel] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const stop = useRef<(() => void) | null>(null);
  useEffect(() => () => stop.current?.(), []);
  const start = async () => {
    setError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: micConstraints(settings) });
      onDevices();
      const ctx = new AudioContext();
      const src = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 1024;
      src.connect(analyser);
      const buf = new Float32Array(analyser.fftSize);
      let raf = 0;
      const tick = () => {
        analyser.getFloatTimeDomainData(buf);
        setLevel(rmsDb(buf));
        raf = requestAnimationFrame(tick);
      };
      tick();
      stop.current = () => {
        cancelAnimationFrame(raf);
        stream.getTracks().forEach((x) => x.stop());
        void ctx.close();
        setLevel(null);
        stop.current = null;
      };
    } catch (e) {
      const name = e instanceof DOMException ? e.name : '';
      setError(name === 'NotAllowedError' || name === 'SecurityError' ? 'denied' : 'missing');
    }
  };
  const threshold = thresholdDb(settings.sensitivity);
  const open = level !== null && level >= threshold;
  return (
    <div className="flex flex-col gap-2" data-testid="voice-mic-test">
      <div className="flex items-center gap-3">
        <Button
          size="sm"
          onClick={() => (stop.current ? stop.current() : void start())}
          data-testid="voice-mic-test-button"
        >
          <Mic className="size-4" aria-hidden />
          {level === null ? t('voice.testMic') : t('voice.stopTest')}
        </Button>
        <div
          className="bg-surface-3 relative h-2 flex-1 overflow-hidden rounded-full"
          role="meter"
          aria-label={t('voice.level')}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(meterValue(level ?? -100) * 100)}
        >
          <span
            className={`absolute inset-y-0 start-0 ${open ? 'bg-success' : 'bg-fg-subtle'}`}
            style={{ width: `${meterValue(level ?? -100) * 100}%` }}
          />
          <span
            className="bg-warning absolute inset-y-0 w-0.5"
            style={{ insetInlineStart: `${meterValue(threshold) * 100}%` }}
            aria-hidden
          />
        </div>
      </div>
      {error && (
        <p className="text-warning text-xs" role="alert">
          {error === 'denied' ? t('voice.micDenied') : t('voice.micMissing')}
        </p>
      )}
    </div>
  );
}

/** Press a key (or a mouse side button) to choose push-to-talk's. */
function KeyCapture() {
  const { t } = useTranslation('settings');
  const pttKey = useVoiceSettings((s) => s.pttKey);
  const set = useVoiceSettings((s) => s.set);
  const [listening, setListening] = useState(false);
  useEffect(() => {
    if (!listening) return;
    const key = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.code !== 'Escape') set({ pttKey: e.code });
      setListening(false);
    };
    const mouse = (e: MouseEvent) => {
      const code = mouseCode(e.button);
      if (!code) return;
      e.preventDefault();
      set({ pttKey: code });
      setListening(false);
    };
    window.addEventListener('keydown', key, { capture: true });
    window.addEventListener('mousedown', mouse, { capture: true });
    return () => {
      window.removeEventListener('keydown', key, { capture: true });
      window.removeEventListener('mousedown', mouse, { capture: true });
    };
  }, [listening, set]);
  return (
    <Button size="sm" onClick={() => setListening(true)} data-testid="voice-ptt-key">
      {listening ? t('voice.pressKey') : keyLabel(pttKey)}
    </Button>
  );
}

function KeyShown({
  title,
  serverId,
  keyText,
  onClose,
}: {
  title: string;
  serverId: string;
  keyText: string;
  onClose: () => void;
}) {
  const { t } = useTranslation('settings');
  const { t: tc } = useTranslation();
  // The two lines of the plugin's config.toml.
  const snippet = `server_id = "${serverId}"
key = "${keyText}"`;
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      icon={<KeyRound />}
      title={title}
      description={t('voice.servers.keyOnce')}
      closeLabel={tc('actions.close')}
      footer={<Button onClick={onClose}>{tc('actions.done')}</Button>}
    >
      <div className="flex items-start gap-2">
        <pre
          className="bg-surface-3 flex-1 overflow-x-auto rounded-md px-2 py-1.5 text-xs"
          data-testid="voice-server-key"
        >
          {snippet}
        </pre>
        <IconButton
          label={tc('actions.copy')}
          icon={<Copy />}
          onClick={() =>
            void navigator.clipboard
              ?.writeText(snippet)
              .then(() => toast({ tone: 'success', title: tc('actions.copied') }))
          }
        />
      </div>
    </Dialog>
  );
}

const SERVERS_KEY = ['voice-servers'] as const;

function VoiceServers() {
  const { t } = useTranslation('settings');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const { data: me } = useMe();
  const { data: servers } = useQuery({
    queryKey: SERVERS_KEY,
    queryFn: async () => (await cloud()).voiceServers(),
    enabled: !!me && me.age_group === 'adult',
  });
  const [name, setName] = useState('');
  const [shown, setShown] = useState<{ title: string; id: string; key: string } | null>(null);
  const refresh = () => void qc.invalidateQueries({ queryKey: SERVERS_KEY });
  const fail = (e: unknown) =>
    toast({
      tone: 'error',
      title:
        errorCode(e) === 'too_many_voice_servers'
          ? t('voice.servers.tooMany')
          : tc('errors.generic'),
    });
  const create = useMutation({
    mutationFn: async () => (await cloud()).createVoiceServer(name.trim()),
    onSuccess: (r) => {
      setName('');
      setShown({ title: r.server.name, id: r.server.id, key: r.key });
      refresh();
    },
    onError: fail,
  });
  const update = useMutation({
    mutationFn: async (v: { id: string; enabled: boolean }) =>
      (await cloud()).updateVoiceServer(v.id, { enabled: v.enabled }),
    onSuccess: refresh,
    onError: fail,
  });
  const remove = useMutation({
    mutationFn: async (id: string) => (await cloud()).deleteVoiceServer(id),
    onSuccess: refresh,
    onError: fail,
  });
  const rotate = useMutation({
    mutationFn: async (s: VoiceServerInfo) => (await cloud()).rotateVoiceServerKey(s.id),
    onSuccess: (r) => setShown({ title: r.server.name, id: r.server.id, key: r.key }),
    onError: fail,
  });
  if (me && me.age_group !== 'adult') return null;
  return (
    <SettingsCard
      title={t('voice.servers.title')}
      description={t('voice.servers.description')}
      testId="settings-voice-servers"
    >
      <form
        className="flex items-center gap-2 pb-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate();
        }}
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          maxLength={64}
          placeholder={t('voice.servers.namePlaceholder')}
          aria-label={t('voice.servers.namePlaceholder')}
          className="flex-1"
        />
        <Button type="submit" variant="primary" disabled={!name.trim()} loading={create.isPending}>
          {t('voice.servers.add')}
        </Button>
      </form>
      {servers?.map((s) => (
        <SettingRow
          key={s.id}
          label={s.name}
          description={t('voice.servers.status', { count: s.online })}
        >
          <div className="flex items-center gap-2">
            <Switch
              checked={s.enabled}
              aria-label={t('voice.servers.enabled', { name: s.name })}
              onCheckedChange={(on) => update.mutate({ id: s.id, enabled: on })}
            />
            <IconButton
              label={t('voice.servers.newKey')}
              icon={<KeyRound />}
              onClick={() => rotate.mutate(s)}
            />
            <IconButton
              label={t('voice.servers.remove', { name: s.name })}
              icon={<Trash2 />}
              className="text-danger"
              onClick={() => remove.mutate(s.id)}
            />
          </div>
        </SettingRow>
      ))}
      {shown && (
        <KeyShown
          title={shown.title}
          serverId={shown.id}
          keyText={shown.key}
          onClose={() => setShown(null)}
        />
      )}
    </SettingsCard>
  );
}

export function VoiceSection() {
  const { t } = useTranslation('settings');
  const s = useVoiceSettings();
  const { inputs, outputs, refresh } = useDevices();
  const deviceOptions = (list: MediaDeviceInfo[]) => [
    { value: 'default', label: t('voice.systemDefault') },
    ...list
      .filter((d) => d.deviceId && d.deviceId !== 'default')
      .map((d, i) => ({ value: d.deviceId, label: d.label || t('voice.deviceN', { n: i + 1 }) })),
  ];
  return (
    <div className="flex flex-col gap-4">
      <SettingsCard
        title={t('voice.devices')}
        description={t('voice.devicesHint')}
        testId="settings-voice"
      >
        <SettingRow label={t('voice.input')} htmlFor="v-in">
          <Select
            id="v-in"
            value={s.inputDeviceId ?? 'default'}
            options={deviceOptions(inputs)}
            onValueChange={(v) => s.set({ inputDeviceId: v === 'default' ? null : v })}
          />
        </SettingRow>
        <SettingRow label={t('voice.output')} htmlFor="v-out">
          <Select
            id="v-out"
            value={s.outputDeviceId ?? 'default'}
            options={deviceOptions(outputs)}
            onValueChange={(v) => s.set({ outputDeviceId: v === 'default' ? null : v })}
          />
        </SettingRow>
        <SettingRow label={t('voice.outputVolume')}>
          <Slider
            label={t('voice.outputVolume')}
            value={Math.round(s.outputVolume * 100)}
            min={0}
            max={200}
            step={5}
            warnAbove={150}
            formatValue={(v) => `${v} %`}
            onValueChange={(v) => s.set({ outputVolume: v / 100 })}
            className="w-56"
          />
        </SettingRow>
      </SettingsCard>
      <SettingsCard title={t('voice.talk')} description={t('voice.talkHint')}>
        <SettingRow label={t('voice.mode')} htmlFor="v-mode">
          <Select
            id="v-mode"
            value={s.mode}
            options={[
              { value: 'vad', label: t('voice.modeVad') },
              { value: 'ptt', label: t('voice.modePtt') },
            ]}
            onValueChange={(v) => s.set({ mode: v as 'vad' | 'ptt' })}
          />
        </SettingRow>
        {s.mode === 'ptt' ? (
          <SettingRow label={t('voice.pttKey')} description={t('voice.pttKeyHint')}>
            <KeyCapture />
          </SettingRow>
        ) : (
          <SettingRow label={t('voice.sensitivity')} description={t('voice.sensitivityHint')}>
            <Slider
              label={t('voice.sensitivity')}
              value={s.sensitivity}
              min={0}
              max={100}
              step={1}
              onValueChange={(v) => s.set({ sensitivity: v })}
              className="w-56"
            />
          </SettingRow>
        )}
        <SettingRow label={t('voice.test')}>
          <div className="w-80">
            <MicTest onDevices={refresh} />
          </div>
        </SettingRow>
        <SettingRow label={t('voice.noiseSuppression')} htmlFor="v-ns">
          <Switch
            id="v-ns"
            checked={s.noiseSuppression}
            onCheckedChange={(v) => s.set({ noiseSuppression: v })}
          />
        </SettingRow>
        <SettingRow label={t('voice.echoCancellation')} htmlFor="v-ec">
          <Switch
            id="v-ec"
            checked={s.echoCancellation}
            onCheckedChange={(v) => s.set({ echoCancellation: v })}
          />
        </SettingRow>
        <SettingRow label={t('voice.autoGain')} htmlFor="v-agc">
          <Switch
            id="v-agc"
            checked={s.autoGainControl}
            onCheckedChange={(v) => s.set({ autoGainControl: v })}
          />
        </SettingRow>
      </SettingsCard>
      <SettingsCard title={t('voice.inGame')} description={t('voice.inGameHint')}>
        <SettingRow label={t('voice.overlay')} htmlFor="v-ov">
          <Switch id="v-ov" checked={s.overlay} onCheckedChange={(v) => s.set({ overlay: v })} />
        </SettingRow>
        <SettingRow
          label={t('voice.serverVoice')}
          description={t('voice.serverVoiceHint')}
          htmlFor="v-sv"
        >
          <Select
            id="v-sv"
            value={s.serverVoice}
            options={[
              { value: 'ask', label: t('voice.serverAsk') },
              { value: 'auto', label: t('voice.serverAuto') },
              { value: 'off', label: t('voice.serverOff') },
            ]}
            onValueChange={(v) => s.set({ serverVoice: v as 'ask' | 'auto' | 'off' })}
          />
        </SettingRow>
      </SettingsCard>
      <VoiceServers />
    </div>
  );
}
