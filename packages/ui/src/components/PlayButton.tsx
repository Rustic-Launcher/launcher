import { AnimatePresence, motion } from 'motion/react';
import { Download, Play, Square, X } from 'lucide-react';
import { forwardRef, useState, type ButtonHTMLAttributes } from 'react';
import { cn } from '../lib/cn';
import { clamp } from '../lib/format';

export type PlayButtonState = 'play' | 'update' | 'progress' | 'running' | 'disabled';

export interface PlayButtonLabels {
  play: string;
  update: string;
  cancel: string;
  running: string;
  /** Shown while in progress and not hovered, e.g. "Downloading 42 %". */
  progress: string;
}

export interface PlayButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  state: PlayButtonState;
  /** 0..1 progress, `null` for indeterminate. Used when state is `progress`. */
  progress?: number | null;
  labels: PlayButtonLabels;
  size?: 'md' | 'lg';
}

/**
 * The big PLAY button of the bottom bar. In `progress` state it fills like a progress bar and
 * turns into a Cancel button on hover/focus; `running` offers Stop.
 */
export const PlayButton = forwardRef<HTMLButtonElement, PlayButtonProps>(function PlayButton(
  { state, progress = null, labels, size = 'lg', className, disabled, ...props },
  ref,
) {
  const [hover, setHover] = useState(false);
  const inProgress = state === 'progress';
  const showCancel = inProgress && hover;
  const pct = progress == null ? null : clamp(progress, 0, 1);

  let label = labels.play;
  let Icon = Play;
  if (state === 'update') {
    label = labels.update;
    Icon = Download;
  } else if (state === 'running') {
    label = labels.running;
    Icon = Square;
  } else if (inProgress) {
    label = showCancel ? labels.cancel : labels.progress;
    Icon = X;
  }

  return (
    <button
      ref={ref}
      type="button"
      disabled={disabled || state === 'disabled'}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onFocus={() => setHover(true)}
      onBlur={() => setHover(false)}
      data-state={state}
      aria-label={label}
      className={cn(
        'rx-focus-ring group relative isolate inline-flex items-center justify-center gap-3 overflow-hidden rounded-xl font-bold tracking-wide uppercase transition-[background-color,box-shadow,transform] duration-200 ease-out active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-45',
        size === 'lg' ? 'h-14 min-w-56 px-10 text-lg' : 'h-11 min-w-40 px-6 text-base',
        state === 'play' && 'bg-accent text-accent-fg shadow-glow hover:bg-accent-hover',
        state === 'update' && 'bg-accent text-accent-fg shadow-glow hover:bg-accent-hover',
        state === 'running' && 'bg-surface-3 text-fg hover:bg-danger hover:text-white',
        state === 'disabled' && 'bg-surface-3 text-fg-muted',
        inProgress && 'bg-surface-3 text-fg',
        showCancel && 'bg-danger text-white',
        className,
      )}
      {...props}
    >
      {inProgress && !showCancel && (
        <span aria-hidden className="absolute inset-0 -z-10 overflow-hidden">
          {pct == null ? (
            <span className="animate-indeterminate bg-accent/70 absolute inset-y-0 w-1/2" />
          ) : (
            <motion.span
              className="bg-accent absolute inset-y-0 left-0"
              initial={false}
              animate={{ width: `${pct * 100}%` }}
              transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
            />
          )}
        </span>
      )}
      <Icon
        aria-hidden
        className={cn('size-5 shrink-0', state === 'play' && 'fill-current')}
        strokeWidth={2.5}
      />
      <AnimatePresence mode="wait" initial={false}>
        <motion.span
          key={label}
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -6 }}
          transition={{ duration: 0.15 }}
          className={cn(inProgress && !showCancel && 'tabular-nums mix-blend-normal')}
        >
          {label}
        </motion.span>
      </AnimatePresence>
    </button>
  );
});
