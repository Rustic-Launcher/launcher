/** Mock: the hero video cache (ADR 0179). No cache in a browser: the CDN URL plays directly. */
import type { MockHandlers } from '../runtime';

export const heroMediaHandlers: MockHandlers = {
  async hero_media_fetch() {
    return null;
  },
};
