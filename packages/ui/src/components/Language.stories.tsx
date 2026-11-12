import type { Meta, StoryObj } from '@storybook/react-vite';
import { AccentText } from './AccentText';
import { Button } from './Button';
import { Card, CardDescription, CardTitle } from './Card';

const meta: Meta = {
  title: 'Foundations/Language',
  tags: ['autodocs'],
  parameters: { layout: 'padded' },
};
export default meta;
type Story = StoryObj;

/** Mono eyebrow + a title with one serif-italic accent word (`*word*` in the text). */
export const Titles: Story = {
  render: () => (
    <div className="flex flex-col gap-6">
      {['Your *profiles*', 'Find your next *mod*', 'Welcome *back*,'].map((t) => (
        <header key={t} className="flex flex-col gap-1.5">
          <span className="rx-eyebrow">Example</span>
          <h1 className="text-3xl font-semibold tracking-tight">
            <AccentText text={t} />
          </h1>
        </header>
      ))}
    </div>
  ),
};

/** `Card` and `.rx-surface`: gradient edge, grain, inner highlight, pointer spotlight. */
export const Surfaces: Story = {
  render: () => (
    <div className="rx-ambient grid max-w-4xl grid-cols-2 gap-4 rounded-2xl p-6">
      <Card padding="lg" className="flex flex-col gap-2">
        <CardTitle>Solid card</CardTitle>
        <CardDescription>The default card is a premium surface.</CardDescription>
      </Card>
      <Card variant="glass" padding="lg" interactive className="flex flex-col gap-2">
        <CardTitle>Interactive glass</CardTitle>
        <CardDescription>Lifts on hover; the spotlight follows the pointer.</CardDescription>
      </Card>
      <section className="rx-surface col-span-2 flex items-center justify-between rounded-2xl p-5">
        <span className="text-sm">A plain container with `rx-surface`</span>
        <Button size="sm" variant="primary">
          Action
        </Button>
      </section>
    </div>
  ),
};

/** Page entrance: children rise from a light blur, 40 ms apart. */
export const PageEnter: Story = {
  render: () => (
    <div className="rx-page-enter flex max-w-md flex-col gap-3">
      {[1, 2, 3, 4].map((i) => (
        <div key={i} className="rx-surface rounded-xl p-4 text-sm">
          Block {i}
        </div>
      ))}
    </div>
  ),
};
