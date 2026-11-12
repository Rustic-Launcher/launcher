import type { ButtonHTMLAttributes } from 'react';
import { cn } from '../lib/cn';

/**
 * The "sign in with Microsoft" button, styled per Microsoft's identity-platform branding
 * guidelines: the official four-square logo (21 × 21, unaltered colours), Segoe UI Semibold
 * 15 px, 41 px high, 12 px padding and gap, square corners; dark theme #2F2F2F with white text,
 * light theme white with a #8C8C8C border and #5E5E5E text. The label is passed in (localised).
 */
export function MicrosoftLogo({ size = 21 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 21 21" aria-hidden>
      <rect x="1" y="1" width="9" height="9" fill="#F25022" />
      <rect x="11" y="1" width="9" height="9" fill="#7FBA00" />
      <rect x="1" y="11" width="9" height="9" fill="#00A4EF" />
      <rect x="11" y="11" width="9" height="9" fill="#FFB900" />
    </svg>
  );
}

export interface MicrosoftButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  theme?: 'dark' | 'light';
}

export function MicrosoftButton({
  label,
  theme = 'dark',
  className,
  ...props
}: MicrosoftButtonProps) {
  return (
    <button
      type="button"
      {...props}
      className={cn(
        'inline-flex h-[41px] w-full items-center justify-center gap-3 px-3 text-[15px] font-semibold transition-[filter] hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#FFB020] disabled:opacity-60',
        theme === 'dark'
          ? 'bg-[#2F2F2F] text-white'
          : 'border border-[#8C8C8C] bg-white text-[#5E5E5E]',
        className,
      )}
      style={{
        fontFamily: '"Segoe UI", "Segoe UI Web (West European)", -apple-system, sans-serif',
      }}
    >
      <MicrosoftLogo />
      <span>{label}</span>
    </button>
  );
}
