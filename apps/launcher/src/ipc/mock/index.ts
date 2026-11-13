/** Browser mock of the Tauri backend (ADR 0024). Used when `window.__TAURI_INTERNALS__` is absent. */
import type { IpcTransport } from '../client';
import { coreHandlers } from './handlers/core';
import { contentHandlers } from './handlers/content';
import { dnsHandlers } from './handlers/dns';
import { hostingHandlers } from './handlers/hosting';
import { multiplayerHandlers } from './handlers/multiplayer';
import { nativeHandlers } from './handlers/native';
import { serverAdminHandlers } from './handlers/serverAdmin';
import { serversHandlers } from './handlers/servers';
import { fail, on, type MockHandler, type MockHandlers } from './runtime';
import type { CommandName } from '../commands';

const handlers: MockHandlers = {
  ...coreHandlers,
  ...contentHandlers,
  ...nativeHandlers,
  ...multiplayerHandlers,
  ...serversHandlers,
  ...serverAdminHandlers,
  ...hostingHandlers,
  ...dnsHandlers,
};

export function createMockTransport(): IpcTransport {
  return {
    async invoke(cmd, args) {
      const h = handlers[cmd] as MockHandler<typeof cmd> | undefined;
      if (!h) fail('unavailable', `mock: command ${cmd} not implemented`);
      return h(args);
    },
    listen(event, handler) {
      return on(event, handler);
    },
  };
}

/** Commands the mock implements (used by a unit test to keep IPC.md honest). */
export function mockedCommands(): CommandName[] {
  return Object.keys(handlers) as CommandName[];
}
