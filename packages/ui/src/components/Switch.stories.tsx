import type { Meta, StoryObj } from '@storybook/react-vite';
import { useState } from 'react';
import { Checkbox, Switch, SwitchRow } from './Switch';

const meta: Meta<typeof Switch> = {
  title: 'Components/Switch & Checkbox',
  component: Switch,
  tags: ['autodocs'],
  args: { 'aria-label': 'Telemetry' },
};
export default meta;
type Story = StoryObj<typeof Switch>;

export const Default: Story = {};
export const Checked: Story = { args: { defaultChecked: true } };
export const Disabled: Story = { args: { disabled: true, defaultChecked: true } };

export const Rows: Story = {
  render: () => (
    <div className="divide-border w-[480px] divide-y">
      <SwitchRow
        label="Share anonymous usage data"
        description="Off by default. Helps us fix crashes."
      />
      <SwitchRow label="Start with the computer" defaultChecked />
      <SwitchRow
        label="Hardware acceleration"
        description="Requires a restart."
        disabled
        defaultChecked
      />
    </div>
  ),
};

export const Checkboxes: StoryObj<typeof Checkbox> = {
  render: function Render() {
    const [a, setA] = useState(false);
    const [b, setB] = useState(true);
    const all = a && b ? true : a || b ? 'indeterminate' : false;
    return (
      <div className="flex flex-col gap-3 text-sm">
        <div className="flex items-center gap-2">
          <Checkbox
            id="cb-all"
            checked={all}
            onCheckedChange={(v) => {
              setA(v === true);
              setB(v === true);
            }}
          />
          <label htmlFor="cb-all">All consents</label>
        </div>
        <div className="ml-6 flex items-center gap-2">
          <Checkbox id="cb-a" checked={a} onCheckedChange={(v) => setA(v === true)} />
          <label htmlFor="cb-a">I want the item delivered immediately</label>
        </div>
        <div className="ml-6 flex items-center gap-2">
          <Checkbox id="cb-b" checked={b} onCheckedChange={(v) => setB(v === true)} />
          <label htmlFor="cb-b">I lose my withdrawal right</label>
        </div>
        <div className="text-fg-muted flex items-center gap-2">
          <Checkbox id="cb-d" disabled />
          <label htmlFor="cb-d">Disabled</label>
        </div>
      </div>
    );
  },
};
