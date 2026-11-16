/**
 * Voice UI (ADR 0176): the room's panel (join, mute, deafen, who speaks, per-member volume,
 * local mute, report) and the top bar's pill while in a room.
 */
import { Avatar, Button, cn, Dialog, IconButton, Select, Slider, toast } from '@rustic/ui';
import {
  Flag,
  Headphones,
  HeadphoneOff,
  Mic,
  MicOff,
  PhoneOff,
  Radio,
  Volume2,
  VolumeX,
  X,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { UserReportReason, VoiceRoomKind } from '@/cloud/types';
import { voiceCommand } from './bridge';
import { useVoiceSettings } from './settings';
import { useVoice, type VoiceParticipant } from './store';

/** The error's sentence, if it has one. */
export function useVoiceErrorText(): string | null {
  const { t } = useTranslation();
  const error = useVoice((s) => s.error);
  if (!error) return null;
  const known: Record<string, string> = {
    voice_unavailable: t('voice.errors.voice_unavailable'),
    voice_not_allowed: t('voice.errors.voice_not_allowed'),
    mic_denied: t('voice.errors.mic_denied'),
    mic_missing: t('voice.errors.mic_missing'),
    connect_failed: t('voice.errors.connect_failed'),
    kicked: t('voice.errors.kicked'),
    not_on_voice_server: t('voice.errors.not_on_voice_server'),
  };
  return known[error] ?? known.connect_failed!;
}

/** Mute and deafen, as two icon toggles. */
export function MuteDeafen({ size = 'md' }: { size?: 'sm' | 'md' }) {
  const { t } = useTranslation();
  const muted = useVoice((s) => s.muted);
  const deafened = useVoice((s) => s.deafened);
  const listenOnly = useVoice((s) => s.listenOnly);
  return (
    <>
      <IconButton
        size={size}
        label={muted ? t('voice.unmute') : t('voice.mute')}
        aria-pressed={muted}
        disabled={listenOnly}
        data-testid="voice-mute"
        icon={muted || listenOnly ? <MicOff /> : <Mic />}
        className={cn(muted && 'text-danger')}
        onClick={() => voiceCommand({ type: 'mute' })}
      />
      <IconButton
        size={size}
        label={deafened ? t('voice.undeafen') : t('voice.deafen')}
        aria-pressed={deafened}
        data-testid="voice-deafen"
        icon={deafened ? <HeadphoneOff /> : <Headphones />}
        className={cn(deafened && 'text-danger')}
        onClick={() => voiceCommand({ type: 'deafen' })}
      />
    </>
  );
}

function SpeakingAvatar({ name, speaking }: { name: string; speaking: boolean }) {
  return (
    <span
      className={cn(
        'rounded-full ring-2 ring-offset-1 ring-offset-transparent transition-shadow',
        speaking ? 'ring-success' : 'ring-transparent',
      )}
    >
      <Avatar name={name} size="xs" />
    </span>
  );
}

function ReportDialog({
  who,
  kind,
  open,
  onOpenChange,
}: {
  who: VoiceParticipant;
  kind: VoiceRoomKind;
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  const { t } = useTranslation();
  const [reason, setReason] = useState<UserReportReason>('harassment');
  const [busy, setBusy] = useState(false);
  const send = async () => {
    setBusy(true);
    try {
      await (
        await cloud()
      ).reportUser({
        user_id: who.id,
        reason,
        context: kind === 'server' ? 'server_voice' : 'party_voice',
      });
      // A reported speaker is muted for the reporter at once.
      if (!useVoiceSettings.getState().localMutes[who.id])
        useVoiceSettings.getState().toggleLocalMute(who.id);
      toast({ tone: 'success', title: t('voice.reported', { name: who.name }) });
      onOpenChange(false);
    } catch {
      toast({ tone: 'error', title: t('errors.generic') });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      tone="destructive"
      icon={<Flag />}
      title={t('voice.reportTitle', { name: who.name })}
      description={t('voice.reportBody')}
      closeLabel={t('actions.close')}
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{t('actions.cancel')}</Button>
          <Button variant="danger" loading={busy} onClick={() => void send()}>
            {t('voice.report')}
          </Button>
        </>
      }
    >
      <Select
        value={reason}
        onValueChange={(v) => setReason(v as UserReportReason)}
        options={(['harassment', 'inappropriate_content', 'spam', 'other'] as const).map((r) => ({
          value: r,
          label: t(`voice.reasons.${r}`),
        }))}
      />
    </Dialog>
  );
}

/** One speaker: who, speaking ring, their volume for me, local mute, report. */
function ParticipantRow({ p, kind }: { p: VoiceParticipant; kind: VoiceRoomKind }) {
  const { t } = useTranslation();
  const volume = useVoiceSettings((s) => s.volumes[p.id] ?? 1);
  const localMuted = useVoiceSettings((s) => !!s.localMutes[p.id]);
  const setVolume = useVoiceSettings((s) => s.setVolume);
  const toggleLocalMute = useVoiceSettings((s) => s.toggleLocalMute);
  const proximity = useVoice((s) => s.proximity);
  const [expanded, setExpanded] = useState(false);
  const [reporting, setReporting] = useState(false);
  return (
    <li className="flex flex-col gap-1 rounded-lg px-2 py-1" data-testid={`voice-member-${p.id}`}>
      <div className="flex items-center gap-2">
        <SpeakingAvatar name={p.name} speaking={p.speaking && !localMuted} />
        <button
          type="button"
          className="rx-focus-ring flex min-w-0 flex-1 flex-col items-start text-left"
          aria-expanded={expanded}
          onClick={() => setExpanded((x) => !x)}
        >
          <span className="truncate text-sm font-medium">{p.name}</span>
          <span className="text-fg-subtle text-2xs">
            {p.speaking && !localMuted
              ? t('voice.speaking')
              : p.micMuted
                ? t('voice.micMuted')
                : proximity && p.audible === 0
                  ? t('voice.outOfRange')
                  : ' '}
          </span>
        </button>
        {p.micMuted && <MicOff className="text-fg-subtle size-3.5" aria-hidden />}
        <IconButton
          size="sm"
          label={
            localMuted
              ? t('voice.unmuteFor', { name: p.name })
              : t('voice.muteFor', { name: p.name })
          }
          aria-pressed={localMuted}
          icon={localMuted ? <VolumeX /> : <Volume2 />}
          className={cn(localMuted && 'text-danger')}
          onClick={() => toggleLocalMute(p.id)}
        />
        <IconButton
          size="sm"
          label={t('voice.reportAria', { name: p.name })}
          icon={<Flag />}
          onClick={() => setReporting(true)}
        />
      </div>
      {expanded && (
        <Slider
          label={t('voice.memberVolume', { name: p.name })}
          value={Math.round(volume * 100)}
          min={0}
          max={200}
          step={5}
          warnAbove={150}
          formatValue={(v) => `${v} %`}
          onValueChange={(v) => setVolume(p.id, v / 100)}
        />
      )}
      <ReportDialog who={p} kind={kind} open={reporting} onOpenChange={setReporting} />
    </li>
  );
}

/** The voice part of the party (or of a server room): join, controls, members. */
export function VoicePanel({ kind, allowed = true }: { kind: VoiceRoomKind; allowed?: boolean }) {
  const { t } = useTranslation();
  const v = useVoice();
  const errorText = useVoiceErrorText();
  const here = v.kind === kind && v.status !== 'idle';
  const connected = here && (v.status === 'connected' || v.status === 'reconnecting');
  return (
    <section
      className="rx-surface flex flex-col gap-2 rounded-xl p-3"
      aria-label={t('voice.title')}
      data-testid={`voice-panel-${kind}`}
    >
      <div className="flex items-center gap-2">
        <Radio
          className={cn('size-4', connected ? 'text-success' : 'text-fg-subtle')}
          aria-hidden
        />
        <h3 className="flex-1 text-xs font-semibold">
          {t('voice.title')}
          {connected && v.proximity && (
            <span className="text-fg-subtle ms-1.5 font-normal">
              {t('voice.proximityOn', { range: v.range })}
            </span>
          )}
        </h3>
        {connected ? (
          <>
            <MuteDeafen size="sm" />
            <IconButton
              size="sm"
              label={t('voice.leave')}
              icon={<PhoneOff />}
              data-testid="voice-leave"
              className="text-danger"
              onClick={() => voiceCommand({ type: 'leave' })}
            />
          </>
        ) : (
          <Button
            size="sm"
            variant="primary"
            data-testid="voice-join"
            disabled={!allowed}
            loading={here && v.status === 'connecting'}
            onClick={() => voiceCommand({ type: 'join', kind })}
          >
            {t('voice.join')}
          </Button>
        )}
      </div>
      {!allowed && <p className="text-fg-subtle text-xs">{t('voice.errors.voice_not_allowed')}</p>}
      {errorText && (v.kind === kind || v.status === 'idle') && (
        <p className="text-warning text-xs" role="status" data-testid="voice-error">
          {errorText}
        </p>
      )}
      {connected && (
        <ul className="-mx-2 flex flex-col">
          <li className="flex items-center gap-2 px-2 py-1">
            <SpeakingAvatar name={t('voice.you')} speaking={v.selfSpeaking} />
            <span className="flex-1 text-sm font-medium">{t('voice.you')}</span>
            <span className="text-fg-subtle text-2xs" data-testid="voice-self-state">
              {v.listenOnly
                ? t('voice.listenOnly')
                : v.muted
                  ? t('voice.micMuted')
                  : v.selfSpeaking
                    ? t('voice.speaking')
                    : ''}
            </span>
          </li>
          {v.participants.map((p) => (
            <ParticipantRow key={p.id} p={p} kind={kind} />
          ))}
          {v.participants.length === 0 && (
            <li className="text-fg-subtle px-2 py-1 text-xs">{t('voice.alone')}</li>
          )}
        </ul>
      )}
    </section>
  );
}

/** The top bar's pill: in a room (mute, deafen, leave), or a server's offer to join its voice. */
export function VoiceBar() {
  const { t } = useTranslation();
  const v = useVoice();
  const connected = v.status === 'connected' || v.status === 'reconnecting';
  if (!connected && v.serverOffer) {
    return (
      <div
        className="bg-accent-soft text-accent-text flex h-8 items-center gap-1.5 rounded-lg ps-2.5 pe-1 text-xs"
        data-testid="voice-offer"
      >
        <Radio className="size-3.5" aria-hidden />
        <span className="max-w-48 truncate">
          {t('voice.serverOffer', { server: v.serverOffer })}
        </span>
        <Button
          size="sm"
          variant="primary"
          className="h-6 px-2"
          onClick={() => voiceCommand({ type: 'join', kind: 'server' })}
        >
          {t('voice.join')}
        </Button>
        <IconButton
          size="sm"
          label={t('actions.close')}
          icon={<X />}
          onClick={() => voiceCommand({ type: 'dismiss-offer' })}
        />
      </div>
    );
  }
  if (!connected) return null;
  const speaking = v.participants.filter((p) => p.speaking);
  return (
    <div
      className="bg-surface-2 flex h-8 items-center gap-1 rounded-lg ps-2.5 pe-1 text-xs"
      data-testid="voice-bar"
      aria-label={t('voice.title')}
    >
      <Radio
        className={cn('size-3.5', v.status === 'connected' ? 'text-success' : 'text-warning')}
        aria-hidden
      />
      <span className="max-w-40 truncate font-medium">
        {v.kind === 'server' ? (v.serverName ?? t('voice.serverRoom')) : t('voice.partyRoom')}
      </span>
      <span className="text-fg-subtle flex items-center gap-0.5" aria-live="polite">
        {speaking.slice(0, 3).map((p) => (
          <SpeakingAvatar key={p.id} name={p.name} speaking />
        ))}
      </span>
      <MuteDeafen size="sm" />
      <IconButton
        size="sm"
        label={t('voice.leave')}
        icon={<PhoneOff />}
        className="text-danger"
        onClick={() => voiceCommand({ type: 'leave' })}
      />
    </div>
  );
}
