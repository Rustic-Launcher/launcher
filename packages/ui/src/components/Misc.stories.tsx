import type { Meta, StoryObj } from '@storybook/react-vite';
import { Monitor, Moon, Sun } from 'lucide-react';
import { useEffect, useState } from 'react';
import { LineChart, ScrollArea, SegmentedControl, Stepper } from './Misc';

const meta: Meta<typeof SegmentedControl> = {
  title: 'Components/Misc',
  component: SegmentedControl,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof SegmentedControl>;

export const Segmented: Story = {
  render: function Render() {
    const [v, setV] = useState('dark');
    return (
      <div className="flex flex-col gap-4">
        <SegmentedControl
          label="Theme"
          value={v}
          onValueChange={setV}
          options={[
            { value: 'dark', label: 'Dark', icon: <Moon /> },
            { value: 'light', label: 'Light', icon: <Sun /> },
            { value: 'system', label: 'System', icon: <Monitor /> },
          ]}
        />
        <SegmentedControl
          label="Range"
          size="sm"
          value="1h"
          onValueChange={() => undefined}
          options={[
            { value: '15m', label: '15 min' },
            { value: '1h', label: '1 h' },
            { value: '24h', label: '24 h', disabled: true },
          ]}
        />
      </div>
    );
  },
};

export const StepperStory: StoryObj<typeof Stepper> = {
  name: 'Stepper',
  render: function Render() {
    const [c, setC] = useState(1);
    return (
      <div className="flex w-[760px] flex-col gap-4">
        <Stepper
          steps={['Name', 'Where to host', 'Type & version', 'World', 'Ready']}
          current={c}
          stepLabel={(i, n) => `Step ${i} of ${n}`}
        />
        <div className="flex gap-2 text-sm">
          <button
            type="button"
            className="border-border rounded border px-2 py-1"
            onClick={() => setC((x) => Math.max(0, x - 1))}
          >
            Previous
          </button>
          <button
            type="button"
            className="border-border rounded border px-2 py-1"
            onClick={() => setC((x) => Math.min(4, x + 1))}
          >
            Next
          </button>
        </div>
      </div>
    );
  },
};

function useLive(base: number, spread: number, n = 60) {
  const [v, setV] = useState(() =>
    Array.from({ length: n }, (_, i) => base + Math.sin(i / 5) * spread),
  );
  useEffect(() => {
    const t = setInterval(
      () => setV((a) => [...a.slice(1), base + (Math.random() - 0.5) * spread * 2]),
      600,
    );
    return () => clearInterval(t);
  }, [base, spread]);
  return v;
}

export const Charts: StoryObj<typeof LineChart> = {
  render: function Render() {
    const tps = useLive(19.6, 0.4);
    const mspt = useLive(28, 14);
    const ram = useLive(2900, 300);
    const cpu = useLive(35, 15);
    return (
      <div className="grid w-[860px] grid-cols-2 gap-6">
        <figure className="border-border rounded-xl border p-4">
          <figcaption className="mb-2 text-sm">TPS (max 20)</figcaption>
          <LineChart
            label="TPS over the last minutes"
            series={[{ id: 'tps', values: tps }]}
            min={0}
            max={20}
          />
        </figure>
        <figure className="border-border rounded-xl border p-4">
          <figcaption className="mb-2 text-sm">MSPT (50 ms threshold)</figcaption>
          <LineChart
            label="Milliseconds per tick"
            series={[{ id: 'mspt', values: mspt, color: 'var(--rl-info)' }]}
            threshold={50}
            max={70}
          />
        </figure>
        <figure className="border-border rounded-xl border p-4">
          <figcaption className="mb-2 text-sm">RAM and CPU (two series)</figcaption>
          <LineChart
            label="RAM and CPU"
            series={[
              { id: 'ram', values: ram.map((x) => x / 40) },
              { id: 'cpu', values: cpu, color: 'var(--rl-success)' },
            ]}
            area={false}
          />
        </figure>
        <figure className="border-border rounded-xl border p-4">
          <figcaption className="mb-2 text-sm">Empty (server stopped)</figcaption>
          <LineChart label="No data" series={[{ id: 'none', values: [] }]} />
        </figure>
      </div>
    );
  },
};

export const Scroll: StoryObj<typeof ScrollArea> = {
  render: () => (
    <ScrollArea className="border-border h-48 w-72 rounded-xl border">
      <ul className="p-3 text-sm">
        {Array.from({ length: 30 }, (_, i) => (
          <li key={i} className="py-1">
            [12:0{i % 10}:14] [Server thread/INFO]: Player{i} joined the game
          </li>
        ))}
      </ul>
    </ScrollArea>
  ),
};
