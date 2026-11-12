import type { Meta, StoryObj } from '@storybook/react-vite';
import { Copy, FolderOpen, MoreHorizontal, Pencil, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { Button } from './Button';
import {
  Dropdown,
  DropdownCheckboxItem,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownSeparator,
  DropdownTrigger,
  Popover,
  PopoverContent,
  PopoverTrigger,
} from './Dropdown';
import { IconButton } from './IconButton';

const meta: Meta<typeof Dropdown> = {
  title: 'Components/Dropdown & Popover',
  component: Dropdown,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Dropdown>;

export const Menu: Story = {
  render: function Render() {
    const [fav, setFav] = useState(true);
    return (
      <Dropdown>
        <DropdownTrigger asChild>
          <IconButton
            label="Instance actions"
            icon={<MoreHorizontal />}
            variant="secondary"
            tooltip={false}
          />
        </DropdownTrigger>
        <DropdownContent align="start">
          <DropdownLabel>Survie entre amis</DropdownLabel>
          <DropdownItem icon={<Pencil />} shortcut="F2">
            Rename
          </DropdownItem>
          <DropdownItem icon={<Copy />}>Duplicate</DropdownItem>
          <DropdownItem icon={<FolderOpen />}>Open folder</DropdownItem>
          <DropdownItem disabled>Export (soon)</DropdownItem>
          <DropdownCheckboxItem checked={fav} onCheckedChange={setFav}>
            Pinned
          </DropdownCheckboxItem>
          <DropdownSeparator />
          <DropdownItem icon={<Trash2 />} destructive>
            Delete
          </DropdownItem>
        </DropdownContent>
      </Dropdown>
    );
  },
};

export const PopoverPanel: Story = {
  render: () => (
    <Popover>
      <PopoverTrigger asChild>
        <Button>Notifications</Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-80 p-4">
        <p className="text-sm font-semibold">Notifications</p>
        <p className="text-fg-muted mt-2 text-sm">You are all caught up.</p>
      </PopoverContent>
    </Popover>
  ),
};
