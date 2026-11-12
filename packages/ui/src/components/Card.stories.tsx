import type { Meta, StoryObj } from '@storybook/react-vite';
import { CheckCircle2, Hammer, XCircle } from 'lucide-react';
import { Badge } from './Badge';
import { Button } from './Button';
import { Card, CardDescription, CardHeader, CardTitle } from './Card';

const meta: Meta<typeof Card> = {
  title: 'Components/Card',
  component: Card,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Card>;

const body = (
  <>
    <CardHeader>
      <div>
        <CardTitle>Survie entre amis</CardTitle>
        <CardDescription>Horizon 0.2.14 (Rustic edition) · 14 mods</CardDescription>
      </div>
      <Badge variant="warning">Update</Badge>
    </CardHeader>
    <p className="text-fg-muted text-sm">312 h played · last played yesterday</p>
  </>
);

export const Variants: Story = {
  render: () => (
    <div className="grid grid-cols-3 gap-4">
      <Card variant="solid">{body}</Card>
      <Card variant="glass">{body}</Card>
      <Card variant="outline">{body}</Card>
    </div>
  ),
};

export const Interactive: Story = {
  render: () => (
    <Card interactive className="w-80" tabIndex={0} role="button" aria-label="Open instance">
      {body}
    </Card>
  ),
};

export const Padding: Story = {
  render: () => (
    <div className="flex gap-4">
      {(['none', 'sm', 'md', 'lg'] as const).map((p) => (
        <Card key={p} padding={p}>
          <span className="text-sm">padding {p}</span>
        </Card>
      ))}
    </div>
  ),
};

export const BadgeVariants: StoryObj<typeof Badge> = {
  name: 'Badge',
  render: () => (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-2">
        {(['neutral', 'accent', 'success', 'warning', 'danger', 'info', 'solid'] as const).map(
          (v) => (
            <Badge key={v} variant={v}>
              {v}
            </Badge>
          ),
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        <Badge size="md" variant="accent">
          Medium
        </Badge>
        <Badge variant="success" icon={<Hammer />}>
          Builds on your Horizon
        </Badge>
        <Badge variant="info" icon={<CheckCircle2 />}>
          Compatible
        </Badge>
        <Badge variant="danger" icon={<XCircle />}>
          Incompatible: Java Edition only
        </Badge>
      </div>
    </div>
  ),
};

export const WithAction: Story = {
  render: () => (
    <Card variant="glass" padding="lg" className="w-96">
      <CardTitle>Rustic Relay</CardTitle>
      <CardDescription className="mt-1">
        Share a temporary address, no port forwarding.
      </CardDescription>
      <Button variant="primary" className="mt-4">
        Enable
      </Button>
    </Card>
  ),
};
