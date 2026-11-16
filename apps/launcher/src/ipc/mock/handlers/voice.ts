/**
 * Mock: voice (ADR 0176). The overlay state is kept for the e2e tests
 * (`window.__rusticVoiceOverlay`), and `window.__rusticVoiceGame(event)` plays a game's voice
 * report (push-to-talk, a position, a place) as the companion would.
 */
import type { VoiceGameEvent, VoiceOverlay } from '../../types/voice';
import { emit, type MockHandlers } from '../runtime';

declare global {
  interface Window {
    __rusticVoiceOverlay?: VoiceOverlay;
    __rusticVoiceGame?: (e: Omit<VoiceGameEvent, 'instance_id'>) => void;
  }
}

if (typeof window !== 'undefined') {
  window.__rusticVoiceGame = (e) => emit('voice-game', { instance_id: 'inst_survival', ...e });
}

export const voiceHandlers: MockHandlers = {
  async voice_overlay_set({ overlay }) {
    window.__rusticVoiceOverlay = overlay;
  },
};
