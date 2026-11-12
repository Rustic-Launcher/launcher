import type { Meta, StoryObj } from '@storybook/react-vite';
import { useState } from 'react';
import { Slider } from './Slider';

const gb = (mb: number) => `${mb / 1024} GB`;

const meta: Meta<typeof Slider> = {
  title: 'Components/Slider',
  component: Slider,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Slider>;

/** RAM allocation: 32 GB machine, warning above half of it. */
export const Ram: Story = {
  render: function Render() {
    const [v, setV] = useState(6144);
    return (
      <div className="flex w-[520px] flex-col gap-2">
        <div className="flex justify-between text-sm">
          <span>Memory</span>
          <span className="tabular-nums">{gb(v)}</span>
        </div>
        <Slider
          label="Memory"
          value={v}
          onValueChange={setV}
          min={1024}
          max={32768}
          step={512}
          formatValue={gb}
          warnAbove={16384}
          marks={[1024, 8192, 16384, 24576, 32768].map((m) => ({ value: m }))}
        />
      </div>
    );
  },
};

export const Disabled: Story = {
  render: () => (
    <div className="w-[520px]">
      <Slider
        label="Memory"
        value={4096}
        onValueChange={() => undefined}
        min={1024}
        max={16384}
        step={512}
        formatValue={gb}
        disabled
      />
    </div>
  ),
};
