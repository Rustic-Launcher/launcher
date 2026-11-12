import type { Meta, StoryObj } from '@storybook/react-vite';
import { ArrowRight, Download, Trash2 } from 'lucide-react';
import { Button } from './Button';

const meta: Meta<typeof Button> = {
  title: 'Components/Button',
  component: Button,
  tags: ['autodocs'],
  args: { children: 'Install', variant: 'primary', size: 'md' },
  argTypes: {
    variant: {
      control: 'select',
      options: ['primary', 'secondary', 'ghost', 'outline', 'danger', 'soft', 'link'],
    },
    size: { control: 'select', options: ['sm', 'md', 'lg', 'xl'] },
  },
};
export default meta;
type Story = StoryObj<typeof Button>;

export const Playground: Story = {};

export const Variants: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="primary">Primary</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="outline">Outline</Button>
      <Button variant="soft">Soft</Button>
      <Button variant="danger">Danger</Button>
      <Button variant="link">Link</Button>
    </div>
  ),
};

export const Sizes: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button size="sm">Small</Button>
      <Button size="md">Medium</Button>
      <Button size="lg">Large</Button>
      <Button size="xl" variant="primary">
        Extra large
      </Button>
    </div>
  ),
};

export const WithIcons: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="primary" leftIcon={<Download />}>
        Download
      </Button>
      <Button rightIcon={<ArrowRight />}>Continue</Button>
      <Button variant="danger" leftIcon={<Trash2 />}>
        Delete server
      </Button>
    </div>
  ),
};

export const Loading: Story = { args: { loading: true, children: 'Installing' } };

export const Disabled: Story = {
  render: () => (
    <div className="flex flex-wrap gap-3">
      <Button variant="primary" disabled>
        Primary
      </Button>
      <Button disabled>Secondary</Button>
      <Button variant="danger" disabled>
        Danger
      </Button>
    </div>
  ),
};

export const AsChildLink: Story = {
  render: () => (
    <Button asChild variant="outline">
      <a href="#docs">Open the docs</a>
    </Button>
  ),
};
