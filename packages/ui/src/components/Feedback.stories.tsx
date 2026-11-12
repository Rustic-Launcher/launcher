import type { Meta, StoryObj } from '@storybook/react-vite';
import { AlertTriangle, PackageOpen } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Button } from './Button';
import { EmptyState, Progress, Separator, Skeleton } from './Feedback';
import { toast } from './Toast';

const meta: Meta<typeof Progress> = {
  title: 'Components/Feedback',
  component: Progress,
  tags: ['autodocs'],
};
export default meta;
type Story = StoryObj<typeof Progress>;

export const ProgressStates: Story = {
  render: () => (
    <div className="flex w-[480px] flex-col gap-5">
      <Progress value={0.42} label="Download" valueText="42 %" />
      <Progress value={null} label="Preparing" />
      <Progress value={0.8} label="Success" tone="success" size="md" />
      <Progress value={0.6} label="Warning" tone="warning" size="md" />
      <Progress value={0.3} label="Error" tone="danger" size="md" />
      <Progress value={0} label="Empty" size="xs" />
    </div>
  ),
};

/** The global download bar pinned on top of the bottom bar. */
export const GlobalDownloadBar: Story = {
  render: function Render() {
    const [p, setP] = useState(0);
    useEffect(() => {
      const t = setInterval(() => setP((v) => (v >= 1 ? 0 : v + 0.02)), 120);
      return () => clearInterval(t);
    }, []);
    return (
      <div className="border-border bg-bg-elevated relative w-full max-w-3xl rounded-lg border">
        <div className="absolute inset-x-0 top-0">
          <Progress
            value={p}
            size="xs"
            label="Background downloads"
            className="rounded-none bg-transparent"
          />
        </div>
        <div className="text-fg-muted flex h-16 items-center px-4 text-sm">
          2 downloads in progress · 12.4 MB/s
        </div>
      </div>
    );
  },
};

export const Skeletons: StoryObj<typeof Skeleton> = {
  render: () => (
    <div className="flex flex-col gap-6">
      <div className="grid w-[720px] grid-cols-3 gap-4">
        {[0, 1, 2].map((i) => (
          <div key={i} className="border-border flex flex-col gap-3 rounded-xl border p-4">
            <Skeleton className="size-11 rounded-xl" />
            <Skeleton className="h-4 w-3/4" />
            <Skeleton className="h-3 w-1/2" />
          </div>
        ))}
      </div>
      <div className="flex w-[480px] flex-col gap-2">
        {[0, 1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-12 rounded-lg" />
        ))}
      </div>
    </div>
  ),
};

export const Empty: StoryObj<typeof EmptyState> = {
  render: () => (
    <div className="grid w-[900px] grid-cols-2 gap-4">
      <EmptyState
        icon={<PackageOpen />}
        title="No mods yet"
        description="Browse compatible mods for this instance."
        action={<Button variant="primary">Browse content</Button>}
      />
      <EmptyState
        tone="error"
        icon={<AlertTriangle />}
        title="Could not load this"
        description="The content source did not answer."
        action={<Button size="sm">Retry</Button>}
      />
      <EmptyState title="Nothing here" description="Text only, no icon." className="col-span-2" />
    </div>
  ),
};

export const Toasts: Story = {
  render: () => (
    <div className="flex flex-wrap gap-2">
      <Button
        onClick={() =>
          toast({
            title: 'Horizon 0.2.15 is available',
            description: 'Update from the bottom bar.',
          })
        }
      >
        Info
      </Button>
      <Button onClick={() => toast({ tone: 'success', title: 'Lumen Renderer installed' })}>
        Success
      </Button>
      <Button
        onClick={() =>
          toast({ tone: 'warning', title: 'Low disk space', description: '3.2 GB left on C:' })
        }
      >
        Warning
      </Button>
      <Button
        onClick={() =>
          toast({ tone: 'error', title: 'The game could not start', description: 'Exit code 1' })
        }
      >
        Error
      </Button>
      <Button
        onClick={() =>
          toast({
            tone: 'success',
            title: 'Server created',
            action: { label: 'Open', onClick: () => undefined },
          })
        }
      >
        With action
      </Button>
      <Button variant="ghost" onClick={() => toast.clear()}>
        Clear all
      </Button>
    </div>
  ),
};

export const Separators: StoryObj<typeof Separator> = {
  render: () => (
    <div className="flex h-16 w-96 flex-col gap-3">
      <Separator />
      <div className="flex h-8 items-center gap-3 text-sm">
        A <Separator vertical /> B
      </div>
    </div>
  ),
};
