import type { Meta, StoryObj } from '@storybook/react-vite';
import { Globe } from 'lucide-react';
import { useState } from 'react';
import { Field, Input, Kbd, SearchInput, Textarea } from './Input';

const meta: Meta<typeof Input> = {
  title: 'Components/Input',
  component: Input,
  tags: ['autodocs'],
  args: { placeholder: 'Server name', 'aria-label': 'Server name' },
};
export default meta;
type Story = StoryObj<typeof Input>;

export const Default: Story = {};
export const WithIcon: Story = {
  args: { leftIcon: <Globe />, placeholder: 'play.example.com', 'aria-label': 'Address' },
};
export const Invalid: Story = { args: { invalid: true, defaultValue: 'bad name!' } };
export const Disabled: Story = { args: { disabled: true, defaultValue: 'Locked' } };

export const Search: StoryObj<typeof SearchInput> = {
  render: function Render() {
    const [v, setV] = useState('');
    return (
      <div className="w-96">
        <SearchInput
          value={v}
          onValueChange={setV}
          clearLabel="Clear"
          shortcut="Ctrl K"
          placeholder="Search mods"
          aria-label="Search mods"
        />
      </div>
    );
  },
};

export const FieldWithHint: StoryObj<typeof Field> = {
  render: () => (
    <div className="flex w-96 flex-col gap-4">
      <Field label="Server name" hint="Shown in the multiplayer list.">
        {(p) => <Input {...p} placeholder="Les Copains SMP" />}
      </Field>
      <Field label="Seed" error="A seed is at most 32 characters.">
        {(p) => <Input {...p} defaultValue="this-seed-is-far-too-long-to-be-accepted" invalid />}
      </Field>
      <Field label="Message of the day">{(p) => <Textarea {...p} placeholder="Welcome!" />}</Field>
      <Field label="Disabled">{(p) => <Textarea {...p} disabled defaultValue="Read only" />}</Field>
    </div>
  ),
};

export const Keys: StoryObj<typeof Kbd> = {
  render: () => (
    <div className="flex gap-2">
      <Kbd>Ctrl</Kbd>
      <Kbd>K</Kbd>
      <Kbd>Esc</Kbd>
    </div>
  ),
};
