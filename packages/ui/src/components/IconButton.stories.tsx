import type { Meta, StoryObj } from '@storybook/react-vite';
import { Bell, Settings, Trash2, UserPlus } from 'lucide-react';
import { IconButton } from './IconButton';
import { Button } from './Button';
import { Tooltip } from './Tooltip';

const meta: Meta<typeof IconButton> = {
  title: 'Components/IconButton',
  component: IconButton,
  tags: ['autodocs'],
  args: { label: 'Settings', icon: <Settings /> },
};
export default meta;
type Story = StoryObj<typeof IconButton>;

export const Default: Story = {};

export const Variants: Story = {
  render: () => (
    <div className="flex items-center gap-3">
      <IconButton label="Ghost" icon={<Settings />} />
      <IconButton label="Secondary" variant="secondary" icon={<UserPlus />} />
      <IconButton label="Primary" variant="primary" icon={<UserPlus />} />
      <IconButton label="Delete" variant="danger" icon={<Trash2 />} />
    </div>
  ),
};

export const Sizes: Story = {
  render: () => (
    <div className="flex items-center gap-3">
      <IconButton size="sm" label="Small" icon={<Settings />} />
      <IconButton size="md" label="Medium" icon={<Settings />} />
      <IconButton size="lg" label="Large" icon={<Settings />} />
    </div>
  ),
};

export const WithBadge: Story = {
  render: () => (
    <div className="flex items-center gap-4">
      <IconButton label="Notifications" icon={<Bell />} badge={3} />
      <IconButton label="Many notifications" icon={<Bell />} badge={24} />
      <IconButton label="New activity" icon={<Bell />} badge />
    </div>
  ),
};

export const WithShortcut: Story = {
  args: { label: 'Settings', shortcut: 'Alt+,', tooltipSide: 'right' },
};

export const Disabled: Story = { args: { disabled: true } };

export const TooltipOnAnyElement: Story = {
  render: () => (
    <Tooltip content="Opens the global search" shortcut="Ctrl K" side="bottom">
      <Button>Search</Button>
    </Tooltip>
  ),
};
