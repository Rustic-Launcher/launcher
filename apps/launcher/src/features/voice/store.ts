/**
 * The voice engine's live state (ADR 0176). The main window runs the engine and owns this state;
 * the Social window gets a copy through `voice-state` events (see `bridge.ts`).
 */
import { create } from 'zustand';
import type { VoiceRoomKind } from '@/cloud/types';

export type VoiceStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting';

/**
 * Why voice stopped or could not start: an API code (`voice_unavailable`, `voice_not_allowed`,
 * `not_found`), `mic_denied` (no microphone permission: listening still works), `mic_missing`,
 * `connect_failed`, `kicked`.
 */
export type VoiceErrorCode = string;

export interface VoiceParticipant {
  id: string;
  name: string;
  speaking: boolean;
  /** Their microphone is muted (by them). */
  micMuted: boolean;
  /** The volume they are heard at now (0-1 before the listener's own volume), for proximity. */
  audible: number;
  /** They report a position in game. */
  inWorld: boolean;
}

export interface VoiceState {
  status: VoiceStatus;
  error: VoiceErrorCode | null;
  kind: VoiceRoomKind | null;
  /** A server room's name. */
  serverName: string | null;
  proximity: boolean;
  range: number;
  muted: boolean;
  deafened: boolean;
  selfSpeaking: boolean;
  /** Listening only (the microphone was refused or is missing). */
  listenOnly: boolean;
  participants: VoiceParticipant[];
  /** The game runs and reports where the player is. */
  inGame: boolean;
  /** The server the game is on offers proximity voice (its name). */
  serverOffer: string | null;
}

export const VOICE_IDLE: VoiceState = {
  status: 'idle',
  error: null,
  kind: null,
  serverName: null,
  proximity: false,
  range: 48,
  muted: false,
  deafened: false,
  selfSpeaking: false,
  listenOnly: false,
  participants: [],
  inGame: false,
  serverOffer: null,
};

export const useVoice = create<VoiceState>()(() => ({ ...VOICE_IDLE }));

/** Commands the UI sends the engine (from any window). */
export type VoiceCommand =
  | { type: 'join'; kind: VoiceRoomKind }
  | { type: 'leave' }
  | { type: 'mute'; on?: boolean }
  | { type: 'deafen'; on?: boolean }
  | { type: 'ptt'; down: boolean }
  | { type: 'dismiss-offer' }
  /** The settings changed in another window (devices, mode, volumes...). */
  | { type: 'settings' };
