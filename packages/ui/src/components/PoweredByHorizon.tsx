import { cn } from '../lib/cn';
import horizonLogo from '../assets/horizon-logo.png';

/**
 * Rustic's one line of positioning, with Horizon's logo small beside it: "Rustic est une
 * plateforme de jeu social, propulsée par le moteur Horizon". Horizon (MIT, github.com/
 * horizon-rust/horizon) is credited, never used as Rustic's brand: the logo appears only here,
 * where the engine is named (the website's footer, the launcher's About, the READMEs).
 */
export function PoweredByHorizon({ text, className }: { text: string; className?: string }) {
  return (
    <p className={cn('text-fg-muted inline-flex items-center gap-1.5 text-sm', className)}>
      <span>{text}</span>
      <img
        src={horizonLogo}
        alt="Horizon"
        width={16}
        height={16}
        draggable={false}
        className="size-4 shrink-0 rounded-[4px] opacity-90"
        data-horizon-logo
      />
    </p>
  );
}
