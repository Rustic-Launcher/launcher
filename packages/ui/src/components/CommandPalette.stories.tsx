import type { Meta, StoryObj } from '@storybook/react-vite';
import { Gamepad2, House, Puzzle, Server, Settings, UserRound } from 'lucide-react';
import { useState } from 'react';
import { Button } from './Button';
import { CommandPalette, type CommandGroup } from './CommandPalette';
import { Kbd } from './Input';

const noop = () => undefined;
const groups: CommandGroup[] = [
  {
    id: 'pages',
    heading: 'Pages',
    items: [
      { id: 'home', label: 'Home', icon: <House />, hint: 'Alt+1', onSelect: noop },
      { id: 'play', label: 'Play', icon: <Gamepad2 />, hint: 'Alt+2', onSelect: noop },
      { id: 'settings', label: 'Settings', icon: <Settings />, onSelect: noop },
    ],
  },
  {
    id: 'content',
    heading: 'Content',
    items: [
      { id: 'lumen', label: 'Lumen Renderer', icon: <Puzzle />, hint: 'Horizon', onSelect: noop },
    ],
  },
  {
    id: 'friends',
    heading: 'Friends',
    items: [
      {
        id: 'lea',
        label: 'Léa',
        keywords: ['lea_builds'],
        icon: <UserRound />,
        hint: 'In game',
        onSelect: noop,
      },
    ],
  },
  {
    id: 'servers',
    heading: 'Servers',
    items: [
      {
        id: 'copains',
        label: 'Les Copains SMP',
        icon: <Server />,
        hint: 'My server',
        onSelect: noop,
      },
    ],
  },
];

const meta: Meta<typeof CommandPalette> = {
  title: 'Components/CommandPalette',
  component: CommandPalette,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof CommandPalette>;

function Demo({
  g = groups,
  loading = false,
  initial = '',
}: {
  g?: CommandGroup[];
  loading?: boolean;
  initial?: string;
}) {
  const [open, setOpen] = useState(true);
  const [q, setQ] = useState(initial);
  return (
    <>
      <Button onClick={() => setOpen(true)}>
        Open search <Kbd>Ctrl K</Kbd>
      </Button>
      <CommandPalette
        open={open}
        onOpenChange={setOpen}
        groups={g}
        query={q}
        onQueryChange={setQ}
        title="Search"
        placeholder="Search content, friends, servers, settings…"
        emptyText="No results. Try another word."
        loading={loading}
        loadingText="Searching…"
        footer={
          <>
            <span>
              <Kbd>↑↓</Kbd> navigate
            </span>
            <span>
              <Kbd>Enter</Kbd> open
            </span>
          </>
        }
      />
    </>
  );
}

export const Default: Story = { render: () => <Demo /> };
export const Empty: Story = { render: () => <Demo initial="zzzz" /> };
export const Loading: Story = { render: () => <Demo g={[]} loading initial="lum" /> };
