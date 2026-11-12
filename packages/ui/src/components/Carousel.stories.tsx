import type { Meta, StoryObj } from '@storybook/react-vite';
import { Carousel } from './Carousel';

interface Slide {
  id: string;
  title: string;
  body: string;
  hue: number;
}

const slides: Slide[] = [
  {
    id: 'a',
    title: 'Rustic Launcher 0.1 beta',
    body: 'Instances, content, servers in two clicks.',
    hue: 22,
  },
  {
    id: 'b',
    title: 'Horizon 0.2.15',
    body: 'Shader pack hot reload and faster meshing.',
    hue: 205,
  },
  {
    id: 'c',
    title: 'Autumn build contest',
    body: 'Win a cosmetic bundle for your party.',
    hue: 32,
  },
];

const meta: Meta<typeof Carousel<Slide>> = {
  title: 'Components/Carousel',
  component: Carousel,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Carousel<Slide>>;

/** Auto-plays every 7 s, pauses on hover/focus and when the OS asks for reduced motion. Arrow keys switch slides. */
export const News: Story = {
  render: () => (
    <Carousel
      className="h-72 w-[820px]"
      items={slides}
      getKey={(s) => s.id}
      label="News"
      prevLabel="Previous slide"
      nextLabel="Next slide"
      slideLabel={(i, n) => `Slide ${i} of ${n}`}
      renderItem={(s) => (
        <div
          className="flex h-full flex-col justify-end gap-1 p-8 text-white"
          style={{
            background: `radial-gradient(600px 300px at 80% 20%, hsl(${s.hue} 70% 55% / 0.55), transparent), linear-gradient(135deg, hsl(${s.hue} 45% 22%), hsl(${(s.hue + 40) % 360} 40% 10%))`,
          }}
        >
          <h3 className="text-2xl font-bold">{s.title}</h3>
          <p className="text-sm text-white/85">{s.body}</p>
        </div>
      )}
    />
  ),
};

export const SingleSlide: Story = {
  render: () => (
    <Carousel
      className="h-48 w-[600px]"
      items={slides.slice(0, 1)}
      getKey={(s) => s.id}
      label="News"
      prevLabel="Previous"
      nextLabel="Next"
      slideLabel={(i, n) => `Slide ${i} of ${n}`}
      renderItem={(s) => (
        <div className="bg-surface-2 grid h-full place-items-center text-lg">{s.title}</div>
      )}
    />
  ),
};
