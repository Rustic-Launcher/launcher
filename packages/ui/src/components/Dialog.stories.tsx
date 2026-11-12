import type { Meta, StoryObj } from '@storybook/react-vite';
import { ShoppingBag, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { Button } from './Button';
import { Dialog } from './Dialog';
import { Sheet } from './Sheet';
import { Checkbox, SwitchRow } from './Switch';

const meta: Meta<typeof Dialog> = {
  title: 'Components/Dialog & Sheet',
  component: Dialog,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Dialog>;

export const Destructive: Story = {
  render: function Render() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <Button variant="danger" onClick={() => setOpen(true)}>
          Delete server
        </Button>
        <Dialog
          open={open}
          onOpenChange={setOpen}
          tone="destructive"
          icon={<Trash2 />}
          title="Delete “Creative test”?"
          description="The world, its backups and its plugins are removed from this computer. This cannot be undone."
          closeLabel="Close"
          footer={
            <>
              <Button onClick={() => setOpen(false)}>Cancel</Button>
              <Button variant="danger" onClick={() => setOpen(false)}>
                Delete
              </Button>
            </>
          }
        />
      </>
    );
  },
};

export const Purchase: Story = {
  render: function Render() {
    const [open, setOpen] = useState(false);
    const [a, setA] = useState(false);
    const [b, setB] = useState(false);
    return (
      <>
        <Button variant="primary" onClick={() => setOpen(true)}>
          Buy for €4.99
        </Button>
        <Dialog
          open={open}
          onOpenChange={setOpen}
          tone="purchase"
          icon={<ShoppingBag />}
          title="Buy Autumn Lantern"
          description="€4.99, VAT included. Purely visual, no gameplay effect."
          closeLabel="Close"
          footer={
            <>
              <Button onClick={() => setOpen(false)}>Cancel</Button>
              <Button variant="primary" disabled={!a || !b} onClick={() => setOpen(false)}>
                Pay €4.99
              </Button>
            </>
          }
        >
          <div className="flex flex-col gap-3 text-sm">
            <div className="flex items-start gap-2.5">
              <Checkbox
                id="consent-now"
                checked={a}
                onCheckedChange={(v) => setA(v === true)}
                className="mt-0.5"
              />
              <label htmlFor="consent-now">I ask for the item to be delivered immediately.</label>
            </div>
            <div className="flex items-start gap-2.5">
              <Checkbox
                id="consent-waiver"
                checked={b}
                onCheckedChange={(v) => setB(v === true)}
                className="mt-0.5"
              />
              <label htmlFor="consent-waiver">
                I acknowledge that I lose my 14-day right of withdrawal once it is delivered.
              </label>
            </div>
          </div>
        </Dialog>
      </>
    );
  },
};

export const SheetRight: Story = {
  render: function Render() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <Button onClick={() => setOpen(true)}>Instance settings</Button>
        <Sheet
          open={open}
          onOpenChange={setOpen}
          title="Survie entre amis"
          description="Horizon 0.2.14 (Rustic edition)"
          closeLabel="Close"
          footer={
            <>
              <Button onClick={() => setOpen(false)}>Cancel</Button>
              <Button variant="primary" onClick={() => setOpen(false)}>
                Save
              </Button>
            </>
          }
        >
          <div className="divide-border divide-y">
            <SwitchRow label="Fullscreen" />
            <SwitchRow label="Show cosmetics" defaultChecked />
          </div>
        </Sheet>
      </>
    );
  },
};

export const SheetLeft: Story = {
  render: function Render() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <Button onClick={() => setOpen(true)}>Filters</Button>
        <Sheet
          open={open}
          onOpenChange={setOpen}
          side="left"
          width={360}
          title="Filters"
          closeLabel="Close"
        >
          <p className="text-fg-muted text-sm">Sources, categories and sort order.</p>
        </Sheet>
      </>
    );
  },
};
