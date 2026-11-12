/**
 * Opens the Social view: a separate, detachable window in the desktop build (Tauri
 * `WebviewWindow` labelled "social", ADR 0028), the in-app overlay in the browser build.
 */
import { isNative } from '@/ipc';
import { useUi, type SocialTab } from '@/stores/ui';

export const SOCIAL_WINDOW_LABEL = 'social';

export async function openSocial(
  opts: { tab?: SocialTab; chatWith?: string | null } = {},
): Promise<void> {
  const ui = useUi.getState();
  ui.setSocial({
    socialTab: opts.tab ?? ui.socialTab,
    socialChatWith: opts.chatWith !== undefined ? opts.chatWith : ui.socialChatWith,
  });
  if (!isNative()) {
    ui.setSocial({ socialOpen: true });
    return;
  }
  try {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const existing = await WebviewWindow.getByLabel(SOCIAL_WINDOW_LABEL);
    if (existing) {
      await existing.unminimize();
      await existing.setFocus();
      return;
    }
    const hash = new URLSearchParams();
    if (opts.tab) hash.set('tab', opts.tab);
    if (opts.chatWith) hash.set('with', opts.chatWith);
    const q = hash.toString();
    new WebviewWindow(SOCIAL_WINDOW_LABEL, {
      url: `index.html#/social${q ? `?${q}` : ''}`,
      title: 'Social',
      width: 1040,
      height: 680,
      minWidth: 760,
      minHeight: 480,
      decorations: false,
      center: true,
    });
  } catch {
    // Window creation refused (capability) or unavailable: fall back to the overlay.
    ui.setSocial({ socialOpen: true });
  }
}
