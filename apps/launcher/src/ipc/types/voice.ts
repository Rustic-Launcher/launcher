/** Voice (ADR 0176): the companion's lines and the in-game overlay. See IPC.md "Voice". */

/** What the in-game overlay shows (`voice_overlay_set`). */
export interface VoiceOverlay {
  /** In a voice room now. */
  active: boolean;
  /** The companion reports the player's position (proximity is on). */
  positions: boolean;
  /** Push-to-talk's key as the game names it (`key.keyboard.v`), null in voice activity mode. */
  ptt_key: string | null;
  /** The room's label ("Party", a server's name). */
  room: string;
  /** "You", in the launcher's language. */
  you: string;
  self_speaking: boolean;
  self_muted: boolean;
  deafened: boolean;
  /** At most 32. */
  members: { name: string; speaking: boolean; muted: boolean }[];
  /** Draw the overlay (voice can stay on with the overlay hidden). */
  show: boolean;
}

/** `voice-game`: a running game's voice report (the companion's lines, the place, the exit). */
export interface VoiceGameEvent {
  instance_id: string;
  /** The push-to-talk key went down (true) or up (false) in game. */
  ptt?: boolean;
  /** Where the player is (`{server, dim, x, y, z, yaw}`), null out of a world. */
  position?: unknown;
  place?: 'menu' | 'singleplayer' | 'server';
  /** The server's host (lowercase) when `place` is `server`. */
  server?: string;
  /** The game exited. */
  exited?: boolean;
}
