/**
 * Where the voice engine runs and how every window talks to it (ADR 0176).
 *
 * The engine lives in the main window (it keeps running while the game is open, the Social
 * window comes and goes). In the desktop build the Social window sends it commands as
 * `voice-command` events and gets its state back as `voice-state` events (Tauri events between
 * the app's own windows); in the browser build everything is one window.
 */
import type { QueryClient } from '@tanstack/react-query';
import { cloud } from '@/cloud/client';
import type { Party } from '@/cloud/types';
import { invoke, isNative, listen } from '@/ipc';
import { sk } from '@/queries/social';
import i18n from '@/i18n';
import { VoiceEngine, type VoiceGameEvent, type VoiceOverlay } from './engine';
import { mouseCode } from './keys';
import { bindVoiceSettingsSync, useVoiceSettings } from './settings';
import { useVoice, type VoiceCommand, type VoiceState } from './store';

let engine: VoiceEngine | null = null;
let emitEvent: ((name: string, payload: unknown) => void) | null = null;

/** Sends a command to the engine, wherever it runs. */
export function voiceCommand(cmd: VoiceCommand | { type: 'sync' }): void {
  if (engine) {
    void handle(engine, cmd);
    return;
  }
  emitEvent?.('voice-command', cmd);
}

async function handle(e: VoiceEngine, cmd: VoiceCommand | { type: 'sync' }) {
  switch (cmd.type) {
    case 'join':
      return e.join(cmd.kind);
    case 'leave':
      return e.leave();
    case 'mute':
      return e.setMuted(cmd.on);
    case 'deafen':
      return e.setDeafened(cmd.on);
    case 'ptt':
      return e.setPtt(cmd.down);
    case 'dismiss-offer':
      return e.dismissOffer();
    case 'settings':
      await useVoiceSettings.persist.rehydrate();
      return e.settingsChanged();
    case 'sync':
      emitEvent?.('voice-state', useVoice.getState());
      return;
  }
}

function editable(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  return !!el && (el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName));
}

/** Push-to-talk while a launcher window has the focus (the game reports its own key). */
function bindPttKeys(send: (down: boolean) => void): () => void {
  const matches = (code: string) => {
    const s = useVoiceSettings.getState();
    return s.mode === 'ptt' && code === s.pttKey;
  };
  const key = (down: boolean) => (ev: KeyboardEvent) => {
    if (ev.repeat || !matches(ev.code)) return;
    // Typing the key in a text field is typing, not talking.
    if (down && editable(ev.target) && ev.key.length === 1) return;
    send(down);
  };
  const mouse = (down: boolean) => (ev: MouseEvent) => {
    const code = mouseCode(ev.button);
    if (code && matches(code)) send(down);
  };
  const kd = key(true);
  const ku = key(false);
  const md = mouse(true);
  const mu = mouse(false);
  const blur = () => send(false);
  window.addEventListener('keydown', kd);
  window.addEventListener('keyup', ku);
  window.addEventListener('mousedown', md);
  window.addEventListener('mouseup', mu);
  window.addEventListener('blur', blur);
  return () => {
    window.removeEventListener('keydown', kd);
    window.removeEventListener('keyup', ku);
    window.removeEventListener('mousedown', md);
    window.removeEventListener('mouseup', mu);
    window.removeEventListener('blur', blur);
  };
}

async function isEngineWindow(): Promise<boolean> {
  if (!isNative()) return true;
  try {
    const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    return getCurrentWebviewWindow().label === 'main';
  } catch {
    return true;
  }
}

/** Starts voice for this window: the engine (main window) or its remote control. */
export async function bindVoice(qc: QueryClient): Promise<() => void> {
  const offs: (() => void)[] = [bindVoiceSettingsSync()];
  if (isNative()) {
    try {
      const ev = await import('@tauri-apps/api/event');
      emitEvent = (name, payload) => void ev.emit(name, payload).catch(() => undefined);
      if (!(await isEngineWindow())) {
        const un = await ev.listen<VoiceState>('voice-state', (e) => useVoice.setState(e.payload));
        offs.push(un);
        offs.push(bindPttKeys((down) => voiceCommand({ type: 'ptt', down })));
        offs.push(useVoiceSettings.subscribe(() => voiceCommand({ type: 'settings' })));
        voiceCommand({ type: 'sync' });
        return () => offs.forEach((o) => o());
      }
      const un = await ev.listen<VoiceCommand | { type: 'sync' }>('voice-command', (e) => {
        if (engine) void handle(engine, e.payload);
      });
      offs.push(un);
    } catch {
      emitEvent = null;
    }
  }

  const e = new VoiceEngine({
    cloud,
    setOverlay: (overlay: VoiceOverlay) =>
      void invoke('voice_overlay_set', { overlay }).catch(() => undefined),
    onState: (s) => emitEvent?.('voice-state', s),
    partyLabel: () => i18n.t('voice.partyRoom'),
    youLabel: () => i18n.t('voice.you'),
  });
  engine = e;
  offs.push(listen('voice-game', (ev: VoiceGameEvent) => e.game(ev)));
  offs.push(bindPttKeys((down) => e.setPtt(down)));
  offs.push(useVoiceSettings.subscribe(() => void e.settingsChanged()));
  // The party's own changes: gone (leave its room), proximity turned on or off by the leader.
  offs.push(
    qc.getQueryCache().subscribe((event) => {
      if (event.query.queryKey[0] !== sk.party[0] || event.type !== 'updated') return;
      const party = event.query.state.data as Party | null | undefined;
      if (useVoice.getState().kind !== 'party') return;
      if (party === null) void e.leave();
      else if (party) e.setRoomMode(party.voice.proximity, party.voice.range);
    }),
  );
  return () => {
    offs.forEach((o) => o());
    void e.dispose();
    if (engine === e) engine = null;
  };
}

/** The engine (tests and the settings' microphone test). */
export function voiceEngine(): VoiceEngine | null {
  return engine;
}
