/** Transient shell UI state. */
import { create } from 'zustand';

export type SocialTab = 'friends' | 'chats';

interface UiState {
  commandOpen: boolean;
  setCommandOpen: (open: boolean) => void;
  /** Social overlay (browser build; the desktop build opens a separate window). */
  socialOpen: boolean;
  socialTab: SocialTab;
  /** Conversation to show when the Social view opens. */
  socialChatWith: string | null;
  setSocial: (p: Partial<Pick<UiState, 'socialOpen' | 'socialTab' | 'socialChatWith'>>) => void;
  /** Focus request inside the Friends tab (from the command palette). */
  friendsFocus: 'search' | null;
  setFriendsFocus: (f: UiState['friendsFocus']) => void;
  /** The "Add a friend" dialog (one host per window: the shell and the Social window). */
  addFriendOpen: boolean;
  setAddFriendOpen: (open: boolean) => void;
}

export const useUi = create<UiState>()((set) => ({
  commandOpen: false,
  setCommandOpen: (commandOpen) => set({ commandOpen }),
  socialOpen: false,
  socialTab: 'friends',
  socialChatWith: null,
  setSocial: (p) => set(p),
  friendsFocus: null,
  setFriendsFocus: (friendsFocus) => set({ friendsFocus }),
  addFriendOpen: false,
  setAddFriendOpen: (addFriendOpen) => set({ addFriendOpen }),
}));
