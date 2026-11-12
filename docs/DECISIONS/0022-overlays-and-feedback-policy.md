# 0022. Overlays and feedback: Dialog only for destructive, purchase and security confirmations

- Status: accepted (frontend, 2026-11-10)

## Context

Launchers tend to stack modals. The brief limits dialogs to destructive actions and purchases; pages must paint
fast with skeletons, not spinners, and long work runs in the background.

## Decision

- `Dialog` (Radix, focus-trapped) is used only for: destructive confirmations (delete instance/server, restore a
  backup, ban, remove friend, delete account), purchases (store, domain purchase) and security confirmations that
  must interrupt (SSH host-key fingerprint on first connection).
- Secondary tasks use a `Sheet` (side panel: new instance, item settings) or inline UI (add friend, rename).
- Loading = `Skeleton` shaped like the content; `Spinner` only inside buttons. Errors = inline `EmptyState`
  (tone error) with Retry; empty = `EmptyState` with the next useful action.
- Background work (downloads, installs, preparation) is reported by IPC events into a global store: a 1-px progress
  bar on top of the bottom bar, a downloads popover in the top bar, and the PLAY button state.
- Toasts (Radix Toast) confirm completed actions; errors that block go inline, never toast-only.

## Consequences

Fewer interruptions; every Dialog has an explicit title, consequence text and a clearly labelled
destructive/primary button. Keyboard and screen readers get Radix's focus management for free.

## Alternatives

Modal-heavy flows (simpler routing, worse UX), spinners (layout shift, slower perceived load).
