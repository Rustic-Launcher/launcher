import type { CloudClient, LinkedIdentities } from '../client';
import type { Me, ParentalControls } from '../types';
import { CloudError } from '../errors';
import { daysAgo, fail, latency, persistGet, persistSet } from '../../ipc/mock/runtime';

const me: Me = {
  id: 'usr_sam',
  username: 'samcrafte',
  display_name: 'Sam',
  email: 'sam@example.com',
  created_at: daysAgo(300),
  age_group: 'adult',
  country: 'FR',
  currency: 'EUR',
  balance: null,
  discoverable: true,
  discoverable_allowed: true,
};
me.discoverable = persistGet<boolean>('discoverable', true);

/** "Appear in suggestions of players I meet" (read by the social mock). */
export function mockDiscoverable(): boolean {
  return me.discoverable && me.discoverable_allowed;
}

let parental: ParentalControls = persistGet<ParentalControls>('parental', {
  enabled: false,
  pin_set: false,
  purchases: 'allowed',
  monthly_limit: null,
  spent_this_month: { amount_minor: 0, currency: 'EUR' },
  chat: 'everyone',
  friend_requests: true,
});
// Sample data: a password account with a linked Microsoft identity (example values).
const linked: LinkedIdentities = {
  has_password: true,
  identities: [{ provider: 'microsoft', email: 'sam@example.invalid', linked_at: daysAgo(40) }],
};

let pin: string | null = persistGet<string | null>('parental_pin', null);

export const accountMock: Partial<CloudClient> = {
  async me() {
    await latency(0.5);
    return structuredClone(me);
  },
  async setDiscoverable(on) {
    await latency(0.5);
    if (on && !me.discoverable_allowed) {
      throw new CloudError('discoverable_not_allowed', 'not available for this account', 403);
    }
    me.discoverable = on;
    persistSet('discoverable', on);
    return structuredClone(me);
  },
  async parental() {
    await latency(0.5);
    return structuredClone(parental);
  },
  async setParental(patch) {
    await latency();
    const { pin: newPin, ...rest } = patch;
    if (newPin !== undefined) {
      if (!/^\d{4,6}$/.test(newPin)) fail('invalid_argument', 'pin');
      pin = newPin;
      persistSet('parental_pin', pin);
    }
    parental = { ...parental, ...rest, pin_set: pin != null };
    persistSet('parental', parental);
    return structuredClone(parental);
  },
  async verifyParentalPin(p) {
    await latency(0.6);
    return pin != null && p === pin;
  },
  async identities() {
    await latency(0.5);
    return structuredClone(linked);
  },
  async unlinkIdentity(provider) {
    await latency();
    if (!linked.has_password) fail('conflict', 'last_sign_in_method');
    linked.identities = linked.identities.filter((i) => i.provider !== provider);
  },
  async exportAccount() {
    await latency(3);
    return {
      url: 'https://rustic.example.invalid/account/export/sam-2026-11-10.zip',
      expires_at: new Date(Date.now() + 7 * 86_400_000).toISOString(),
    };
  },
  async deleteAccount(confirm) {
    await latency(2);
    if (confirm !== me.username) fail('invalid_argument', 'confirmation');
    return { scheduled_for: new Date(Date.now() + 30 * 86_400_000).toISOString() };
  },
  async joinWaitlist() {
    await latency();
    return { position: 1284 };
  },
};
