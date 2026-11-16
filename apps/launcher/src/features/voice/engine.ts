/**
 * The voice engine (ADR 0176): one LiveKit room at a time (the party's, or a Rustic Server's
 * proximity room), in the main window's webview, which keeps running while the game is open.
 *
 * - Microphone: `getUserMedia` with the settings' device and processing (noise suppression,
 *   echo cancellation, automatic gain), published as the room's only track. Voice activity
 *   detection or push-to-talk gate it by enabling the track (silence is sent otherwise, cheap
 *   with DTX); mute also tells the room. A refused microphone leaves the user listening.
 * - Speakers: each remote microphone goes through our own WebAudio graph (gain, stereo panner,
 *   master gain, output device), so the per-member volume, deafen and proximity apply without
 *   touching LiveKit's elements.
 * - Proximity: the companion reports the player's position; the engine sends it to the room
 *   as a lossy data message and mixes every speaker from the two positions (`proximity.ts`).
 * - Nothing is recorded: audio only flows through the SFU to the room's members.
 */
import type * as LiveKitModule from 'livekit-client';
import type {
  LocalTrackPublication,
  Participant,
  RemoteParticipant,
  RemoteTrack,
  Room,
} from 'livekit-client';
import type { CloudClient } from '@/cloud/client';
import { errorCode } from '@/cloud/errors';
import type { VoiceRoomKind, VoiceTokenInfo } from '@/cloud/types';
import type { VoiceGameEvent, VoiceOverlay } from '@/ipc/types/voice';
import { gameKeyOf } from './keys';
import {
  POSITION_TOPIC,
  decodePosition,
  encodePosition,
  mix,
  moved,
  sanitizePosition,
  type GamePosition,
} from './proximity';
import { micConstraints, useVoiceSettings, type VoiceSettings } from './settings';
import { VOICE_IDLE, useVoice, type VoiceParticipant, type VoiceState } from './store';
import { micOpen, rmsDb, thresholdDb, VoiceGate } from './vad';

type LiveKit = typeof LiveKitModule;

export type { VoiceGameEvent, VoiceOverlay };

interface RemoteAudio {
  el: HTMLAudioElement;
  source: MediaStreamAudioSourceNode;
  gain: GainNode;
  pan: StereoPannerNode;
}

export interface EngineDeps {
  cloud: () => Promise<CloudClient>;
  /** Hands the overlay to the game (no-op outside the desktop build). */
  setOverlay: (o: VoiceOverlay) => void;
  /** The engine's state changed (the bridge copies it to the other windows). */
  onState?: (s: VoiceState) => void;
  /** Labels for the overlay ("Party", "You"). */
  partyLabel?: () => string;
  youLabel?: () => string;
}

/** Positions are sent at most this often, and at least once a second while in game. */
const POSITION_MIN_MS = 200;
const POSITION_HEARTBEAT_MS = 1000;
/** The overlay file is refreshed this often while in a room (the companion drops stale ones). */
const OVERLAY_HEARTBEAT_MS = 2000;

export class VoiceEngine {
  private lk: LiveKit | null = null;
  private room: Room | null = null;
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private mic: MediaStream | null = null;
  private micTrack: MediaStreamTrack | null = null;
  private publication: LocalTrackPublication | null = null;
  private analysis: { track: MediaStreamTrack; node: ScriptProcessorNode; sink: GainNode } | null =
    null;
  private gate = new VoiceGate();
  private gateOpen = false;
  private level = -100;
  private pttDown = false;
  private remotes = new Map<string, RemoteAudio>();
  private positions = new Map<string, GamePosition | null>();
  private speaking = new Set<string>();
  private selfPos: GamePosition | null = null;
  private sentPos: GamePosition | null = null;
  private sentAt = 0;
  private gameServer: string | null = null;
  private probed = new Set<string>();
  private timers: ReturnType<typeof setTimeout>[] = [];
  private heartbeat: ReturnType<typeof setInterval> | null = null;
  private overlayDue: ReturnType<typeof setTimeout> | null = null;
  private joining: Promise<void> | null = null;
  private generation = 0;

  constructor(private readonly deps: EngineDeps) {}

  get state(): VoiceState {
    return useVoice.getState();
  }

  private patch(p: Partial<VoiceState>) {
    useVoice.setState(p);
    this.deps.onState?.(useVoice.getState());
    this.overlaySoon();
  }

  private settings(): VoiceSettings {
    return useVoiceSettings.getState();
  }

  // ---------------------------------------------------------------- joining and leaving

  async join(kind: VoiceRoomKind, info?: VoiceTokenInfo): Promise<void> {
    const s = this.state;
    if (s.kind === kind && (s.status === 'connected' || s.status === 'connecting')) return;
    if (this.joining) await this.joining;
    if (this.room) await this.leave();
    this.joining = this.doJoin(kind, info).finally(() => (this.joining = null));
    return this.joining;
  }

  private async doJoin(kind: VoiceRoomKind, given?: VoiceTokenInfo): Promise<void> {
    const gen = ++this.generation;
    this.patch({
      ...VOICE_IDLE,
      inGame: this.state.inGame,
      serverOffer: kind === 'server' ? null : this.state.serverOffer,
      status: 'connecting',
      kind,
    });
    let info: VoiceTokenInfo;
    try {
      info = given ?? (await (await this.deps.cloud()).voiceToken(kind));
    } catch (e) {
      this.patch({ status: 'idle', kind: null, error: errorCode(e) ?? 'connect_failed' });
      return;
    }
    if (gen !== this.generation) return;
    try {
      this.lk ??= await import('livekit-client');
      this.ctx = new AudioContext({ latencyHint: 'interactive' });
      this.master = this.ctx.createGain();
      this.master.connect(this.ctx.destination);
      this.applyOutput();
      const room = new this.lk.Room({
        adaptiveStream: false,
        dynacast: false,
        disconnectOnPageLeave: true,
        stopLocalTrackOnUnpublish: true,
      });
      this.room = room;
      this.wire(room);
      await room.connect(info.url, info.token, { autoSubscribe: true });
    } catch {
      if (gen === this.generation) {
        await this.cleanup();
        this.patch({ status: 'idle', kind: null, error: 'connect_failed' });
      }
      return;
    }
    if (gen !== this.generation) return;
    this.patch({
      status: 'connected',
      kind,
      serverName: info.server_name,
      proximity: info.proximity,
      range: info.range,
    });
    await this.startMic();
    this.resumeAudio();
    for (const p of this.room?.remoteParticipants.values() ?? []) this.attachExisting(p);
    this.refreshParticipants();
    this.heartbeat = setInterval(() => {
      this.sendPosition(false);
      this.overlaySoon();
    }, POSITION_HEARTBEAT_MS);
    this.sendPosition(true);
  }

  async leave(): Promise<void> {
    this.generation++;
    const room = this.room;
    this.room = null;
    if (room) await room.disconnect().catch(() => undefined);
    await this.cleanup();
    this.patch({
      ...VOICE_IDLE,
      inGame: this.state.inGame,
      serverOffer: this.state.serverOffer,
    });
  }

  private async cleanup() {
    if (this.heartbeat) clearInterval(this.heartbeat);
    this.heartbeat = null;
    if (this.analysis) {
      this.analysis.node.onaudioprocess = null;
      this.analysis.node.disconnect();
      this.analysis.sink.disconnect();
      this.analysis.track.stop();
    }
    this.analysis = null;
    this.mic?.getTracks().forEach((t) => t.stop());
    this.mic = null;
    this.micTrack = null;
    this.publication = null;
    for (const id of [...this.remotes.keys()]) this.detach(id);
    this.positions.clear();
    this.speaking.clear();
    this.gate.reset();
    this.gateOpen = false;
    this.sentPos = null;
    const ctx = this.ctx;
    this.ctx = null;
    this.master = null;
    if (ctx) await ctx.close().catch(() => undefined);
  }

  private wire(room: Room) {
    const E = this.lk!.RoomEvent;
    room
      .on(E.TrackSubscribed, (track, _pub, participant) => {
        if (track.kind === 'audio') this.attach(participant, track);
      })
      .on(E.TrackUnsubscribed, (_track, _pub, participant) => {
        this.detach(participant.identity);
        this.refreshParticipants();
      })
      .on(E.ParticipantConnected, () => {
        this.refreshParticipants();
        // A newcomer learns where we are at once.
        this.sendPosition(true);
      })
      .on(E.ParticipantDisconnected, (p) => {
        this.detach(p.identity);
        this.positions.delete(p.identity);
        this.speaking.delete(p.identity);
        this.refreshParticipants();
      })
      .on(E.TrackMuted, () => this.refreshParticipants())
      .on(E.TrackUnmuted, () => this.refreshParticipants())
      .on(E.ActiveSpeakersChanged, (speakers: Participant[]) => {
        this.speaking = new Set(
          speakers.filter((p) => p !== room.localParticipant).map((p) => p.identity),
        );
        this.refreshParticipants();
      })
      .on(E.DataReceived, (payload, participant, _kind, topic) => {
        if (topic !== POSITION_TOPIC || !participant) return;
        const p = decodePosition(payload);
        if (p === undefined) return;
        this.positions.set(participant.identity, p);
        this.remix();
      })
      .on(E.Reconnecting, () => this.patch({ status: 'reconnecting' }))
      .on(E.Reconnected, () => {
        this.patch({ status: 'connected' });
        this.sendPosition(true);
      })
      .on(E.Disconnected, (reason) => {
        if (this.room !== room) return;
        const D = this.lk!.DisconnectReason;
        const error =
          reason === D.PARTICIPANT_REMOVED || reason === D.ROOM_DELETED
            ? 'kicked'
            : reason === D.CLIENT_INITIATED
              ? null
              : 'connect_failed';
        this.room = null;
        void this.cleanup().then(() =>
          this.patch({
            ...VOICE_IDLE,
            inGame: this.state.inGame,
            serverOffer: this.state.serverOffer,
            error,
          }),
        );
      });
  }

  // ---------------------------------------------------------------- microphone

  private async startMic() {
    const room = this.room;
    if (!room || !this.lk || !this.ctx) return;
    const s = this.settings();
    let stream: MediaStream | null = null;
    try {
      if (!navigator.mediaDevices?.getUserMedia) throw new DOMException('', 'NotFoundError');
      stream = await navigator.mediaDevices.getUserMedia({ audio: micConstraints(s) });
    } catch (e) {
      const name = e instanceof DOMException ? e.name : '';
      if ((name === 'OverconstrainedError' || name === 'NotFoundError') && s.inputDeviceId) {
        // The chosen device is gone: the default one.
        stream = await navigator.mediaDevices
          .getUserMedia({ audio: micConstraints({ ...s, inputDeviceId: null }) })
          .catch(() => null);
      }
      if (!stream) {
        const denied = name === 'NotAllowedError' || name === 'SecurityError';
        this.patch({ listenOnly: true, error: denied ? 'mic_denied' : 'mic_missing' });
        return;
      }
    }
    if (this.room !== room) {
      stream.getTracks().forEach((t) => t.stop());
      return;
    }
    this.mic = stream;
    this.micTrack = stream.getAudioTracks()[0] ?? null;
    if (!this.micTrack) return;
    this.micTrack.enabled = false;
    try {
      this.publication = await room.localParticipant.publishTrack(this.micTrack, {
        source: this.lk.Track.Source.Microphone,
        name: 'microphone',
        dtx: true,
        red: true,
      });
    } catch {
      this.patch({ listenOnly: true, error: 'connect_failed' });
      return;
    }
    this.startAnalysis(this.micTrack);
    this.patch({ listenOnly: false, error: null });
    this.applyMic();
  }

  /** Levels for voice activity on a copy of the microphone (the published one may be off). */
  private startAnalysis(track: MediaStreamTrack) {
    if (!this.ctx) return;
    const copy = track.clone();
    copy.enabled = true;
    const src = this.ctx.createMediaStreamSource(new MediaStream([copy]));
    // An audio-thread callback: unlike timers, it is not slowed down when the window is hidden.
    const node = this.ctx.createScriptProcessor(1024, 1, 1);
    const sink = this.ctx.createGain();
    sink.gain.value = 0;
    src.connect(node);
    node.connect(sink);
    sink.connect(this.ctx.destination);
    node.onaudioprocess = (ev) => {
      this.level = rmsDb(ev.inputBuffer.getChannelData(0));
      const threshold = thresholdDb(this.settings().sensitivity);
      const open = this.gate.update(this.level, threshold, performance.now());
      if (open !== this.gateOpen) {
        this.gateOpen = open;
        this.applyMic();
      } else if (this.settings().mode === 'ptt' && this.pttDown) {
        this.applyMic();
      }
    };
    this.analysis = { track: copy, node, sink };
  }

  private applyMic() {
    const s = this.state;
    const settings = this.settings();
    const open = micOpen({
      muted: s.muted,
      deafened: s.deafened,
      mode: settings.mode,
      pttDown: this.pttDown,
      gateOpen: this.gateOpen,
    });
    if (this.micTrack && this.micTrack.enabled !== open) this.micTrack.enabled = open;
    // Push-to-talk: the key held is not speaking yet; the voice is.
    const speaking = open && (settings.mode === 'vad' || this.level > -60);
    if (speaking !== s.selfSpeaking) this.patch({ selfSpeaking: speaking });
  }

  async setMuted(on?: boolean) {
    const muted = on ?? !this.state.muted;
    this.patch({ muted, deafened: muted ? this.state.deafened : false });
    if (this.publication) {
      if (muted) await this.publication.mute().catch(() => undefined);
      else await this.publication.unmute().catch(() => undefined);
    }
    this.applyMic();
    this.remix();
  }

  /** Deafen mutes too (as voice chat apps do); undeafen gives the microphone back. */
  async setDeafened(on?: boolean) {
    const deafened = on ?? !this.state.deafened;
    this.patch({ deafened });
    await this.setMuted(deafened);
    this.patch({ deafened });
    this.remix();
  }

  setPtt(down: boolean) {
    if (this.pttDown === down) return;
    this.pttDown = down;
    this.applyMic();
  }

  /** Devices, processing or volumes changed. */
  async settingsChanged() {
    this.applyOutput();
    this.remix();
    this.applyMic();
    this.overlaySoon();
    const s = this.settings();
    const t = this.micTrack;
    if (!t || !this.publication?.track) return;
    const current = t.getSettings();
    const wanted = micConstraints(s);
    const changed =
      (s.inputDeviceId && current.deviceId !== s.inputDeviceId) ||
      current.noiseSuppression !== wanted.noiseSuppression ||
      current.echoCancellation !== wanted.echoCancellation ||
      current.autoGainControl !== wanted.autoGainControl;
    if (!changed) return;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: wanted });
      const next = stream.getAudioTracks()[0];
      if (!next) return;
      next.enabled = t.enabled;
      await this.publication.track.replaceTrack(next);
      this.mic?.getTracks().forEach((x) => x.stop());
      this.mic = stream;
      this.micTrack = next;
      if (this.analysis) {
        this.analysis.node.onaudioprocess = null;
        this.analysis.node.disconnect();
        this.analysis.track.stop();
      }
      this.startAnalysis(next);
      this.applyMic();
    } catch {
      /* the old microphone stays */
    }
  }

  // ---------------------------------------------------------------- speakers

  private attachExisting(p: RemoteParticipant) {
    for (const pub of p.audioTrackPublications.values()) {
      if (pub.track) this.attach(p, pub.track);
    }
  }

  private attach(participant: RemoteParticipant, track: RemoteTrack) {
    if (!this.ctx || !this.master) return;
    this.detach(participant.identity);
    const stream = new MediaStream([track.mediaStreamTrack]);
    // Chromium only feeds a remote WebRTC stream to WebAudio while a media element plays it.
    const el = new Audio();
    el.muted = true;
    el.srcObject = stream;
    void el.play().catch(() => undefined);
    const source = this.ctx.createMediaStreamSource(stream);
    const gain = this.ctx.createGain();
    const pan = this.ctx.createStereoPanner();
    gain.gain.value = 0;
    source.connect(gain).connect(pan).connect(this.master);
    this.remotes.set(participant.identity, { el, source, gain, pan });
    this.remix();
    this.refreshParticipants();
  }

  private detach(id: string) {
    const r = this.remotes.get(id);
    if (!r) return;
    r.source.disconnect();
    r.gain.disconnect();
    r.pan.disconnect();
    r.el.srcObject = null;
    this.remotes.delete(id);
  }

  private applyOutput() {
    const s = this.settings();
    if (this.master && this.ctx)
      this.master.gain.setTargetAtTime(s.outputVolume, this.ctx.currentTime, 0.05);
    const ctx = this.ctx as (AudioContext & { setSinkId?: (id: string) => Promise<void> }) | null;
    if (ctx?.setSinkId) void ctx.setSinkId(s.outputDeviceId ?? '').catch(() => undefined);
  }

  /** The audio context may wait for a gesture (auto-join): resume on the next click or key. */
  private resumeAudio() {
    const ctx = this.ctx;
    if (!ctx || ctx.state !== 'suspended') return;
    void ctx.resume().catch(() => undefined);
    const resume = () => {
      void this.ctx?.resume().catch(() => undefined);
      window.removeEventListener('pointerdown', resume);
      window.removeEventListener('keydown', resume);
    };
    window.addEventListener('pointerdown', resume);
    window.addEventListener('keydown', resume);
  }

  /** Every speaker's gain and pan, from the room's mode, positions and the listener's volumes. */
  private remix() {
    const ctx = this.ctx;
    if (!ctx) return;
    const s = this.state;
    const settings = this.settings();
    for (const [id, r] of this.remotes) {
      const m = mix({
        proximity: s.proximity,
        strict: s.kind === 'server',
        range: s.range,
        self: this.selfPos,
        other: this.positions.get(id) ?? null,
      });
      const user = settings.localMutes[id] ? 0 : (settings.volumes[id] ?? 1);
      const g = s.deafened ? 0 : m.gain * user;
      r.gain.gain.setTargetAtTime(g, ctx.currentTime, 0.08);
      r.pan.pan.setTargetAtTime(m.pan, ctx.currentTime, 0.08);
    }
    this.refreshParticipants();
  }

  private refreshParticipants() {
    const room = this.room;
    if (!room) return;
    const s = this.state;
    const list: VoiceParticipant[] = [...room.remoteParticipants.values()].map((p) => {
      const pos = this.positions.get(p.identity) ?? null;
      const audible = mix({
        proximity: s.proximity,
        strict: s.kind === 'server',
        range: s.range,
        self: this.selfPos,
        other: pos,
      }).gain;
      const mic = [...p.audioTrackPublications.values()][0];
      return {
        id: p.identity,
        name: p.name || p.identity,
        speaking: this.speaking.has(p.identity) && audible > 0,
        micMuted: !mic || mic.isMuted,
        audible,
        inWorld: !!pos,
      };
    });
    list.sort((a, b) => a.name.localeCompare(b.name));
    const same =
      list.length === s.participants.length &&
      list.every((p, i) => {
        const q = s.participants[i]!;
        return (
          p.id === q.id &&
          p.name === q.name &&
          p.speaking === q.speaking &&
          p.micMuted === q.micMuted &&
          p.inWorld === q.inWorld &&
          Math.abs(p.audible - q.audible) < 0.02
        );
      });
    if (!same) this.patch({ participants: list });
  }

  /** The party's voice settings changed (the leader turned proximity on, the range...). */
  setRoomMode(proximity: boolean, range: number) {
    if (this.state.kind !== 'party') return;
    if (proximity === this.state.proximity && range === this.state.range) return;
    this.patch({ proximity, range });
    this.remix();
    this.sendPosition(true);
  }

  // ---------------------------------------------------------------- the game

  game(e: VoiceGameEvent) {
    if (e.exited) {
      this.selfPos = null;
      this.gameServer = null;
      this.pttDown = false;
      this.applyMic();
      this.patch({ inGame: false, serverOffer: null });
      this.sendPosition(true);
      if (this.state.kind === 'server') void this.leave();
      this.remix();
      return;
    }
    if (e.ptt !== undefined) this.setPtt(e.ptt);
    if (e.position !== undefined) {
      this.selfPos = e.position === null ? null : sanitizePosition(e.position);
      this.sendPosition(false);
      this.remix();
    }
    if (e.place) {
      const server = e.place === 'server' ? (e.server ?? null) : null;
      if (!this.state.inGame) this.patch({ inGame: true });
      if (server !== this.gameServer) {
        this.gameServer = server;
        this.patch({ serverOffer: null });
        if (!server && this.state.kind === 'server') void this.leave();
        if (server) this.probeServer(server);
      }
    }
  }

  /** Whether the server the game joined offers proximity voice (its plugin lists the player). */
  private probeServer(server: string) {
    const mode = this.settings().serverVoice;
    if (mode === 'off' || this.probed.has(server)) return;
    this.probed.add(server);
    // The plugin sends who is online every few seconds: ask after it had time to list us.
    for (const delay of [6000, 20000]) {
      this.timers.push(
        setTimeout(() => {
          if (this.gameServer !== server || this.state.kind === 'server') return;
          if (this.state.serverOffer) return;
          void this.deps
            .cloud()
            .then((c) => c.voiceToken('server'))
            .then((info) => {
              if (this.gameServer !== server) return;
              if (this.settings().serverVoice === 'auto' && this.state.status === 'idle') {
                void this.join('server', info);
              } else {
                this.patch({ serverOffer: info.server_name ?? server });
              }
            })
            .catch(() => undefined);
        }, delay),
      );
    }
  }

  dismissOffer() {
    this.patch({ serverOffer: null });
  }

  private sendPosition(force: boolean) {
    const room = this.room;
    if (!room || this.state.status !== 'connected' || !this.state.proximity) return;
    const now = Date.now();
    const due = now - this.sentAt >= POSITION_HEARTBEAT_MS;
    if (
      !force &&
      !due &&
      (now - this.sentAt < POSITION_MIN_MS || !moved(this.sentPos, this.selfPos))
    )
      return;
    this.sentAt = now;
    this.sentPos = this.selfPos;
    void room.localParticipant
      .publishData(encodePosition(this.selfPos), { reliable: false, topic: POSITION_TOPIC })
      .catch(() => undefined);
  }

  // ---------------------------------------------------------------- the overlay

  private overlaySoon() {
    if (this.overlayDue) return;
    this.overlayDue = setTimeout(() => {
      this.overlayDue = null;
      this.pushOverlay();
    }, 120);
  }

  private pushOverlay() {
    const s = this.state;
    if (!s.inGame) return;
    const settings = this.settings();
    const active = s.status === 'connected' || s.status === 'reconnecting';
    this.deps.setOverlay({
      active,
      positions: active && s.proximity,
      ptt_key: settings.mode === 'ptt' ? gameKeyOf(settings.pttKey) : null,
      room: s.kind === 'server' ? (s.serverName ?? '') : (this.deps.partyLabel?.() ?? 'Party'),
      you: this.deps.youLabel?.() ?? 'You',
      self_speaking: s.selfSpeaking,
      self_muted: s.muted || s.listenOnly,
      deafened: s.deafened,
      members: s.participants
        .filter((p) => s.kind !== 'server' || p.audible > 0)
        .slice(0, 32)
        .map((p) => ({ name: p.name, speaking: p.speaking, muted: p.micMuted })),
      show: settings.overlay,
    });
  }

  /** Clears pending timers (tests, window teardown). */
  async dispose() {
    this.timers.forEach(clearTimeout);
    this.timers = [];
    if (this.overlayDue) clearTimeout(this.overlayDue);
    await this.leave();
  }

  /** Overlay heartbeat interval, exposed for the bridge. */
  static readonly OVERLAY_HEARTBEAT_MS = OVERLAY_HEARTBEAT_MS;
}
