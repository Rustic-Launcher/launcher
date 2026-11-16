/**
 * The push-to-talk key, chosen in the launcher (a `KeyboardEvent.code`, or `Mouse4`/`Mouse5` for
 * a mouse's side buttons) and recognised in game by its vanilla key name (`key.keyboard.v`,
 * `key.mouse.4`), which is what the companion's key hook reports.
 */

const NAMED: Record<string, [game: string, label: string]> = {
  Space: ['space', 'Space'],
  Tab: ['tab', 'Tab'],
  CapsLock: ['caps.lock', 'Caps Lock'],
  ShiftLeft: ['left.shift', 'Left Shift'],
  ShiftRight: ['right.shift', 'Right Shift'],
  ControlLeft: ['left.control', 'Left Ctrl'],
  ControlRight: ['right.control', 'Right Ctrl'],
  AltLeft: ['left.alt', 'Left Alt'],
  AltRight: ['right.alt', 'Right Alt'],
  Backquote: ['grave.accent', '`'],
  Minus: ['minus', '-'],
  Equal: ['equal', '='],
  BracketLeft: ['left.bracket', '['],
  BracketRight: ['right.bracket', ']'],
  Backslash: ['backslash', '\\'],
  Semicolon: ['semicolon', ';'],
  Quote: ['apostrophe', "'"],
  Comma: ['comma', ','],
  Period: ['period', '.'],
  Slash: ['slash', '/'],
  Insert: ['insert', 'Insert'],
  Home: ['home', 'Home'],
  End: ['end', 'End'],
  PageUp: ['page.up', 'Page Up'],
  PageDown: ['page.down', 'Page Down'],
  ArrowUp: ['up', '↑'],
  ArrowDown: ['down', '↓'],
  ArrowLeft: ['left', '←'],
  ArrowRight: ['right', '→'],
};

export const DEFAULT_PTT_KEY = 'KeyV';

/** The game's name for the key, or null when the game has none for it. */
export function gameKeyOf(code: string): string | null {
  const mouse = /^Mouse([3-8])$/.exec(code);
  if (mouse) return `key.mouse.${mouse[1]}`;
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return `key.keyboard.${letter[1]!.toLowerCase()}`;
  const digit = /^Digit([0-9])$/.exec(code);
  if (digit) return `key.keyboard.${digit[1]}`;
  const pad = /^Numpad([0-9])$/.exec(code);
  if (pad) return `key.keyboard.keypad.${pad[1]}`;
  const f = /^F([1-9]|1[0-9]|2[0-5])$/.exec(code);
  if (f) return `key.keyboard.f${f[1]}`;
  const named = NAMED[code];
  return named ? `key.keyboard.${named[0]}` : null;
}

/** A short label for the key. */
export function keyLabel(code: string): string {
  const mouse = /^Mouse([3-8])$/.exec(code);
  if (mouse) return `Mouse ${mouse[1]}`;
  const letter = /^(?:Key|Digit)(.)$/.exec(code);
  if (letter) return letter[1]!;
  const pad = /^Numpad([0-9])$/.exec(code);
  if (pad) return `Num ${pad[1]}`;
  if (/^F\d+$/.test(code)) return code;
  return NAMED[code]?.[1] ?? code;
}

/** The code a mouse button press stands for (side buttons only), if any. */
export function mouseCode(button: number): string | null {
  // MouseEvent.button: 3 = back, 4 = forward, the game's mouse buttons 4 and 5.
  return button === 3 ? 'Mouse4' : button === 4 ? 'Mouse5' : null;
}
