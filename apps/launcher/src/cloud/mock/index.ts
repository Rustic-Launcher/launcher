import type { CloudClient } from '../client';
import { accountMock } from './account';
import { socialMock } from './social';
import { storeMock } from './store';

export function createMockCloud(): CloudClient {
  const impl = { ...socialMock, ...accountMock, ...storeMock };
  return new Proxy(impl, {
    get(target, prop: string) {
      const v = (target as Record<string, unknown>)[prop];
      if (v === undefined && prop !== 'then') {
        return () => Promise.reject(new Error(`cloud mock: ${prop} not implemented`));
      }
      return v;
    },
  }) as CloudClient;
}
