import type { Meta, StoryObj } from '@storybook/react-vite';
import { lazy, Suspense } from 'react';
import type { CosmeticVisual } from './PlayerViewer';

// The canvas is lazy so three.js only loads for these stories.
const PlayerViewer = lazy(() =>
  import('./PlayerViewer').then((m) => ({ default: m.PlayerViewer })),
);

const outfit: CosmeticVisual[] = [
  { slot: 'head', shape: 'crown', color: '#C9A227', accent: '#E8834A' },
  { slot: 'back', shape: 'backpack', color: '#6B4A2E', accent: '#D4A24C' },
  { slot: 'shoulder', shape: 'fox', color: '#E07B39' },
  { slot: 'aura', shape: 'sparks', color: '#F2965F' },
  { slot: 'trail', shape: 'embers', color: '#E8834A' },
];

const meta: Meta<typeof PlayerViewer> = {
  title: '3D/PlayerViewer',
  component: PlayerViewer,
  tags: ['autodocs'],
  args: { label: 'Your character, drag to rotate', animation: 'idle', cosmetics: outfit },
  argTypes: {
    animation: { control: 'select', options: ['idle', 'walk', 'wave', 'dance', 'none'] },
  },
  render: (args) => (
    <div className="border-border bg-surface/40 h-[520px] w-[420px] rounded-2xl border">
      <Suspense fallback={<div className="rx-skeleton animate-shimmer size-full rounded-2xl" />}>
        <PlayerViewer {...args} />
      </Suspense>
    </div>
  ),
};
export default meta;
type Story = StoryObj<typeof PlayerViewer>;

export const Idle: Story = {};
export const Walk: Story = { args: { animation: 'walk' } };
export const Wave: Story = { args: { animation: 'wave' } };
export const Dance: Story = { args: { animation: 'dance', autoRotate: true } };
export const NoCosmetics: Story = { args: { cosmetics: [], animation: 'none' } };
export const Companion: Story = {
  args: {
    cosmetics: [
      { slot: 'companion', shape: 'fox', color: '#8C7A5B' },
      { slot: 'head', shape: 'halo', color: '#F4D35E' },
      { slot: 'top', shape: 'hoodie', color: '#2F3B52', accent: '#E8834A' },
    ],
  },
};
