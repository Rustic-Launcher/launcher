import type { Meta, StoryObj } from '@storybook/react-vite';
import { Avatar, type Presence as PresenceKind } from './Avatar';

const meta: Meta<typeof Avatar> = {
  title: 'Components/Avatar',
  component: Avatar,
  tags: ['autodocs'],
  args: { name: 'Léa', presence: 'online', presenceLabel: 'Online', size: 'md' },
};
export default meta;
type Story = StoryObj<typeof Avatar>;

export const Default: Story = {};

export const Sizes: Story = {
  render: () => (
    <div className="flex items-end gap-4">
      {(['xs', 'sm', 'md', 'lg', 'xl'] as const).map((s) => (
        <Avatar key={s} name="Sam" size={s} presence="in-game" presenceLabel="In game" />
      ))}
    </div>
  ),
};

export const Presence: Story = {
  render: () => (
    <div className="flex items-center gap-6">
      {(['online', 'in-game', 'away', 'busy', 'offline'] as PresenceKind[]).map((p) => (
        <div key={p} className="text-fg-muted flex flex-col items-center gap-2 text-xs">
          <Avatar name={p} presence={p} presenceLabel={p} size="lg" />
          {p}
        </div>
      ))}
    </div>
  ),
};

export const Shapes: Story = {
  render: () => (
    <div className="flex gap-4">
      <Avatar name="Noah" shape="square" size="lg" />
      <Avatar name="Noah" shape="circle" size="lg" />
    </div>
  ),
};

/** A broken image URL falls back to initials on a stable gradient. */
export const ImageFallback: Story = {
  args: { src: 'https://invalid.example/avatar.png', name: 'Inès' },
};
