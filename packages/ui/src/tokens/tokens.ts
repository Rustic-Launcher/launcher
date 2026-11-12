/** Motion and layering tokens for JS (Framer Motion) — mirrors tokens.css. */
export const duration = { fast: 0.15, base: 0.2, slow: 0.25 } as const;
export const easing = {
  /** ease.standard (enter / move) */
  out: [0.2, 0, 0, 1] as [number, number, number, number],
  /** ease.exit (leave, at 80 % of the enter duration) */
  exit: [0.4, 0, 1, 1] as [number, number, number, number],
  inOut: [0.65, 0, 0.35, 1] as [number, number, number, number],
};
export const zIndex = {
  base: 0,
  rail: 10,
  sticky: 20,
  panel: 30,
  dropdown: 40,
  overlay: 50,
  modal: 60,
  popover: 70,
  toast: 80,
  tooltip: 90,
} as const;
export const radius = { xs: 6, sm: 8, md: 12, lg: 14, xl: 16, '2xl': 20 } as const;
/** ease.spring (Locker model turn, drag snap) */
export const spring = { type: 'spring', stiffness: 420, damping: 34 } as const;
/** Spacing scale in px (4 px grid). */
export const space = [0, 4, 8, 12, 16, 20, 24, 32, 40, 48, 64] as const;
export const fontSize = {
  '2xs': 11,
  xs: 12,
  sm: 13,
  base: 14,
  lg: 16,
  xl: 20,
  '2xl': 24,
  '3xl': 30,
  '4xl': 36,
} as const;

/** Standard transition for UI elements. */
export const transition = { duration: duration.base, ease: easing.out };
