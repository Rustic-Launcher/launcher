import type { Meta, StoryObj } from '@storybook/react-vite';
import { useEffect, useState } from 'react';
import { PlayButton, type PlayButtonState } from './PlayButton';

const labels = {
  play: 'Play',
  update: 'Update',
  cancel: 'Cancel',
  running: 'Stop',
  progress: 'Downloading 42 %',
};

const meta: Meta<typeof PlayButton> = {
  title: 'Components/PlayButton',
  component: PlayButton,
  tags: ['autodocs'],
  args: { state: 'play', labels, size: 'lg' },
  argTypes: {
    state: { control: 'select', options: ['play', 'update', 'progress', 'running', 'disabled'] },
    progress: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
  },
};
export default meta;
type Story = StoryObj<typeof PlayButton>;

export const Play: Story = {};
export const Update: Story = { args: { state: 'update' } };
export const ProgressDeterminate: Story = { args: { state: 'progress', progress: 0.42 } };
export const ProgressIndeterminate: Story = {
  args: { state: 'progress', progress: null, labels: { ...labels, progress: 'Preparing…' } },
};
export const Running: Story = { args: { state: 'running' } };
export const Disabled: Story = { args: { state: 'disabled' } };
export const Medium: Story = { args: { size: 'md' } };

/** Click Play: download runs, hover to cancel, then the game "runs" and Stop returns to Play. */
export const Interactive: Story = {
  render: function Render() {
    const [state, setState] = useState<PlayButtonState>('update');
    const [p, setP] = useState(0);
    useEffect(() => {
      if (state !== 'progress') return;
      const t = setInterval(() => {
        setP((v) => {
          if (v >= 1) {
            clearInterval(t);
            setState('running');
            return 1;
          }
          return Math.min(1, v + 0.05);
        });
      }, 150);
      return () => clearInterval(t);
    }, [state]);
    return (
      <div className="flex flex-col items-start gap-3">
        <PlayButton
          state={state}
          progress={p}
          labels={{ ...labels, progress: `Downloading ${Math.round(p * 100)} %` }}
          onClick={() => {
            if (state === 'update' || state === 'play') {
              setP(0);
              setState('progress');
            } else if (state === 'progress') setState('update');
            else if (state === 'running') setState('play');
          }}
        />
        <span className="text-fg-muted text-xs">State: {state}</span>
      </div>
    );
  },
};
