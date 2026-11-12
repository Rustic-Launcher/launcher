import type { Meta, StoryObj } from '@storybook/react-vite';
import { Boxes, Cpu, Server, Sparkles } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from './Badge';
import { Button } from './Button';
import { PremiumCard, PremiumGrid } from './PremiumCard';

const meta: Meta<typeof PremiumCard> = {
  title: 'Components/PremiumCard',
  component: PremiumCard,
  tags: ['autodocs'],
  parameters: { layout: 'padded' },
};
export default meta;
type Story = StoryObj<typeof PremiumCard>;

function Body({ icon, title, text }: { icon: ReactNode; title: string; text: string }) {
  return (
    <div className="flex flex-col gap-3">
      <span
        data-depth="2"
        className="text-accent-text inline-flex size-9 items-center [&_svg]:size-6"
      >
        {icon}
      </span>
      <h3 data-depth="1" className="text-lg font-semibold">
        {title}
      </h3>
      <p className="text-fg-muted text-sm">{text}</p>
    </div>
  );
}

export const Default: Story = {
  render: () => (
    <div className="max-w-sm">
      <PremiumCard>
        <Body
          icon={<Boxes />}
          title="Mods in one click"
          text="Example card: Modrinth, CurseForge and Horizon content, checked for fair play."
        />
      </PremiumCard>
    </div>
  ),
};

export const Variants: Story = {
  render: () => (
    <PremiumGrid className="grid max-w-5xl grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
      <PremiumCard>
        <Body icon={<Boxes />} title="Default" text="Glass, grain, gradient edge." />
      </PremiumCard>
      <PremiumCard variant="featured">
        <Body icon={<Sparkles />} title="Featured" text="Stronger edge, warm corner glow." />
      </PremiumCard>
      <PremiumCard variant="accent">
        <div className="flex flex-col gap-3">
          <h3 data-depth="1" className="text-lg font-semibold">
            Accent plate
          </h3>
          <p className="text-sm opacity-80">Dark text on the accent, AA contrast.</p>
          <div data-depth="2">
            <Button size="sm" variant="secondary">
              Learn more
            </Button>
          </div>
        </div>
      </PremiumCard>
      <PremiumCard variant="compact">
        <div className="flex items-center gap-3">
          <Server className="text-accent-text size-5" aria-hidden />
          <span className="text-sm font-semibold">Compact</span>
          <Badge variant="neutral">example</Badge>
        </div>
      </PremiumCard>
    </PremiumGrid>
  ),
};

/** Move the pointer across the grid: neighbouring borders light up with one shared spotlight. */
export const SharedSpotlight: Story = {
  render: () => (
    <PremiumGrid className="grid max-w-4xl grid-cols-3 gap-3">
      {Array.from({ length: 6 }, (_, i) => (
        <PremiumCard key={i} variant="compact" className="min-h-28">
          <div className="flex items-center gap-2 text-sm">
            <Cpu className="text-fg-muted size-4" aria-hidden /> Tile {i + 1}
          </div>
        </PremiumCard>
      ))}
    </PremiumGrid>
  ),
};

/** Keyboard: tab to the link, the card shows an amber focus glow. */
export const Focusable: Story = {
  render: () => (
    <div className="max-w-sm">
      <PremiumCard as="article">
        <a href="#card" className="text-base font-semibold underline-offset-4 hover:underline">
          A focusable card link
        </a>
        <p className="text-fg-muted mt-2 text-sm">The whole card shows the focus state.</p>
      </PremiumCard>
    </div>
  ),
};

export const Static: Story = {
  render: () => (
    <div className="max-w-sm">
      <PremiumCard interactive={false}>
        <Body icon={<Boxes />} title="No pointer effects" text="For dense lists and forms." />
      </PremiumCard>
    </div>
  ),
};
