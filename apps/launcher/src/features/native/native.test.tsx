import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { invoke, IpcFailure } from '@/ipc';
import type { DownloadTask } from '@/ipc/types/app';
import { useActivity } from '@/stores/activity';
import { buildTime } from './NativeBuildPanel';
import { NativeToolchainDialog, NativeTrustDialog } from './NativeDialogs';
import { taskFraction, waitForTask } from './useNative';

const task = (over: Partial<DownloadTask>): DownloadTask => ({
  id: 't1',
  kind: 'native-build',
  label: 'Native mods: x',
  instance_id: 'inst_survival',
  received_bytes: 0,
  total_bytes: 1000,
  speed_bps: 0,
  state: 'running',
  ...over,
});

describe('native mods', () => {
  it('builds only after the player ticks the trust notice', () => {
    const onAccept = vi.fn();
    render(
      <NativeTrustDialog
        open
        onOpenChange={() => {}}
        modName="FPS Counter"
        instanceName="Survie entre amis"
        sourceUrl="https://github.com/horizon-rust/horizon-mods/tree/main/native/fps-overlay"
        pending={false}
        onAccept={onAccept}
      />,
    );
    expect(screen.getByText(/FPS Counter is a native mod/)).toBeInTheDocument();
    expect(screen.getByText(/not sandboxed/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Read the source code' })).toBeInTheDocument();
    const accept = screen.getByTestId('native-trust-accept');
    expect(accept).toBeDisabled();
    fireEvent.click(accept);
    expect(onAccept).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId('native-trust-check'));
    expect(accept).toBeEnabled();
    fireEvent.click(accept);
    expect(onAccept).toHaveBeenCalledOnce();
  });

  it("names what to install when the launcher can't install Rust itself", () => {
    render(
      <NativeToolchainDialog
        open
        onOpenChange={() => {}}
        pending={false}
        onInstall={() => {}}
        toolchain={{
          state: 'missing',
          kind: null,
          version: '1.99.1',
          reason: 'no C compiler (cc) is installed',
          installable: false,
          prerequisites: ['pkg-config (`sudo apt install pkg-config`)'],
        }}
      />,
    );
    expect(screen.queryByTestId('native-toolchain-install')).toBeNull();
    expect(screen.getByText(/sudo apt install pkg-config/)).toBeInTheDocument();
  });

  it('follows a task until it ends', async () => {
    useActivity.getState().upsertDownload(task({ id: 'a' }));
    const done = waitForTask('a');
    useActivity.getState().upsertDownload(task({ id: 'a', received_bytes: 500 }));
    useActivity.getState().upsertDownload(task({ id: 'a', state: 'done' }));
    await expect(done).resolves.toBe(true);

    useActivity.getState().upsertDownload(task({ id: 'b' }));
    const failed = waitForTask('b');
    useActivity.getState().upsertDownload(task({ id: 'b', state: 'failed', error: 'x' }));
    await expect(failed).resolves.toBe(false);
  });

  it('reads progress and build times', () => {
    expect(taskFraction(task({ received_bytes: 250 }))).toBe(0.25);
    expect(taskFraction(task({ total_bytes: null }))).toBeNull();
    expect(taskFraction(undefined)).toBeNull();
    expect(buildTime(42.4, 'en')).toBe('42s');
    expect(buildTime(825, 'en')).toBe('13m');
  });

  it('refuses a build without the trust notice, then without Rust', async () => {
    const err = await invoke('native_build', { instance_id: 'inst_survival' }).catch(
      (e: unknown) => e,
    );
    expect(err).toBeInstanceOf(IpcFailure);
    expect((err as IpcFailure).details?.reason).toBe('native.consent_required');
    const s = await invoke('native_consent', { instance_id: 'inst_survival', accept: true });
    expect(s.consent).toBe(true);
    const err2 = await invoke('native_build', { instance_id: 'inst_survival' }).catch(
      (e: unknown) => e,
    );
    expect((err2 as IpcFailure).details?.reason).toBe('native.toolchain_missing');
    await invoke('native_consent', { instance_id: 'inst_survival', accept: false });
  });
});
