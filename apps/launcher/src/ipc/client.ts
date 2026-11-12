/**
 * Typed IPC client. Inside Tauri it uses `@tauri-apps/api` (`invoke`/`listen`); in a plain browser
 * (dev, Storybook, Playwright) it falls back to the in-memory mock in ./mock.
 */
import type { CommandArgs, CommandName, CommandResult, EventName, IpcEvents } from './commands';
import type { IpcError } from './types/app';

export type Unlisten = () => void;

export interface IpcTransport {
  invoke<K extends CommandName>(cmd: K, args: CommandArgs<K>): Promise<CommandResult<K>>;
  listen<E extends EventName>(event: E, handler: (payload: IpcEvents[E]) => void): Unlisten;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isNative(): boolean {
  return typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;
}

export class IpcFailure extends Error implements IpcError {
  code: IpcError['code'];
  details?: Record<string, unknown>;
  constructor(e: IpcError) {
    super(e.message);
    this.name = 'IpcFailure';
    this.code = e.code;
    this.details = e.details;
  }
}

function toFailure(e: unknown): IpcFailure {
  if (e instanceof IpcFailure) return e;
  if (e && typeof e === 'object' && 'code' in e && 'message' in e)
    return new IpcFailure(e as IpcError);
  return new IpcFailure({ code: 'internal', message: String(e) });
}

const tauriTransport: IpcTransport = {
  async invoke(cmd, args) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke(cmd, args === undefined ? undefined : { args });
    } catch (e) {
      throw toFailure(e);
    }
  },
  listen(event, handler) {
    let un: (() => void) | null = null;
    let cancelled = false;
    void import('@tauri-apps/api/event').then(({ listen }) =>
      listen(event, (e) => handler(e.payload as never)).then((u) => {
        if (cancelled) u();
        else un = u;
      }),
    );
    return () => {
      cancelled = true;
      un?.();
    };
  },
};

let transport: IpcTransport | null = null;
let pending: Promise<IpcTransport> | null = null;

async function getTransport(): Promise<IpcTransport> {
  if (transport) return transport;
  if (!pending) {
    pending = isNative()
      ? Promise.resolve(tauriTransport)
      : import('./mock').then((m) => m.createMockTransport());
  }
  transport = await pending;
  return transport;
}

/** Replace the transport (tests). */
export function setTransport(t: IpcTransport): void {
  transport = t;
  pending = Promise.resolve(t);
}

export async function invoke<K extends CommandName>(
  cmd: K,
  ...args: CommandArgs<K> extends void ? [] : [CommandArgs<K>]
): Promise<CommandResult<K>> {
  const t = await getTransport();
  try {
    return await t.invoke(cmd, args[0] as CommandArgs<K>);
  } catch (e) {
    throw toFailure(e);
  }
}

export function listen<E extends EventName>(
  event: E,
  handler: (payload: IpcEvents[E]) => void,
): Unlisten {
  let un: Unlisten | null = null;
  let cancelled = false;
  void getTransport().then((t) => {
    if (!cancelled) un = t.listen(event, handler);
  });
  return () => {
    cancelled = true;
    un?.();
  };
}
