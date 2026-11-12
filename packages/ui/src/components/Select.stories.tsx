import type { Meta, StoryObj } from '@storybook/react-vite';
import { useState } from 'react';
import { Select } from './Select';

const versions = [
  { value: 'v0.2.15', label: 'Horizon 0.2.15 (Rustic edition)', description: 'Recommended' },
  { value: 'v0.2.14', label: 'Horizon 0.2.14 (Rustic edition)', description: 'Installed' },
  { value: 'nightly', label: 'Horizon Nightly 2026-11-08', description: 'Unstable' },
  { value: 'v0.1', label: 'Horizon 0.1.9', description: 'No longer supported', disabled: true },
];

const meta: Meta<typeof Select> = {
  title: 'Components/Select',
  component: Select,
  tags: ['autodocs'],
  args: { options: versions, 'aria-label': 'Horizon version' },
};
export default meta;
type Story = StoryObj<typeof Select>;

export const Controlled: Story = {
  render: function Render(args) {
    const [v, setV] = useState('v0.2.15');
    return (
      <div className="w-80">
        <Select {...args} value={v} onValueChange={setV} />
      </div>
    );
  },
};

export const Placeholder: Story = {
  render: (args) => (
    <div className="w-80">
      <Select {...args} placeholder="Choose a version" />
    </div>
  ),
};

export const Small: Story = {
  render: (args) => (
    <div className="w-48">
      <Select
        {...args}
        size="sm"
        defaultValue="EUR"
        aria-label="Currency"
        options={['EUR', 'USD', 'GBP', 'CHF'].map((c) => ({ value: c, label: c }))}
      />
    </div>
  ),
};

export const Disabled: Story = {
  render: (args) => (
    <div className="w-80">
      <Select {...args} defaultValue="v0.2.14" disabled />
    </div>
  ),
};
