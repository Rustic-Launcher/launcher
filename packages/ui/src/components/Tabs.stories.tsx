import type { Meta, StoryObj } from '@storybook/react-vite';
import { Image, Layers, Puzzle, Sparkles } from 'lucide-react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './Tabs';

const meta: Meta<typeof Tabs> = {
  title: 'Components/Tabs',
  component: Tabs,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Tabs>;

const content = (
  <>
    <TabsContent value="mods">
      <p className="text-fg-muted text-sm">14 mods installed.</p>
    </TabsContent>
    <TabsContent value="packs">
      <p className="text-fg-muted text-sm">2 resource packs.</p>
    </TabsContent>
    <TabsContent value="shaders">
      <p className="text-fg-muted text-sm">1 shader pack.</p>
    </TabsContent>
  </>
);

export const Underline: Story = {
  render: () => (
    <Tabs defaultValue="mods" className="w-[560px]">
      <TabsList aria-label="Content kinds">
        <TabsTrigger value="mods" icon={<Puzzle />} count={14}>
          Mods
        </TabsTrigger>
        <TabsTrigger value="packs" icon={<Image />} count={2}>
          Resource packs
        </TabsTrigger>
        <TabsTrigger value="shaders" icon={<Sparkles />}>
          Shaders
        </TabsTrigger>
        <TabsTrigger value="plugins" icon={<Layers />} disabled>
          Plugins
        </TabsTrigger>
      </TabsList>
      {content}
    </Tabs>
  ),
};

export const Pill: Story = {
  render: () => (
    <Tabs defaultValue="packs" variant="pill">
      <TabsList aria-label="Content kinds">
        <TabsTrigger value="mods">Mods</TabsTrigger>
        <TabsTrigger value="packs">Resource packs</TabsTrigger>
        <TabsTrigger value="shaders">Shaders</TabsTrigger>
      </TabsList>
      {content}
    </Tabs>
  ),
};
