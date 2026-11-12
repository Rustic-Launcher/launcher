/** Pure helpers of the "Add a friend" dialog (unit-tested in addFriend.test.ts). */

export type NameKind = 'rustic' | 'minecraft';

/** Same rules as the API: Rustic names 3-20, Minecraft names 3-16, letters, digits and `_`. */
export function validName(kind: NameKind, name: string): boolean {
  return (kind === 'rustic' ? /^[A-Za-z0-9_]{3,20}$/ : /^[A-Za-z0-9_]{3,16}$/).test(name);
}

export type FriendErrorKey =
  | 'notFoundRustic'
  | 'notFoundMinecraft'
  | 'invalidRustic'
  | 'invalidMinecraft'
  | 'cannot_add_friend'
  | 'friend_requests_disabled'
  | 'cannot_add_self'
  | 'already_friends'
  | 'request_pending'
  | 'incoming_request_exists'
  | 'rate_limited'
  | 'generic';

/** i18n key (under `addFriend.errors`) for an API error code. */
export function friendErrorKey(code: string | null, kind: NameKind): FriendErrorKey {
  switch (code) {
    case 'not_found':
      return kind === 'rustic' ? 'notFoundRustic' : 'notFoundMinecraft';
    case 'cannot_add_friend':
    case 'friend_requests_disabled':
    case 'cannot_add_self':
    case 'already_friends':
    case 'request_pending':
    case 'incoming_request_exists':
    case 'rate_limited':
      return code;
    case 'invalid_input':
      return kind === 'rustic' ? 'invalidRustic' : 'invalidMinecraft';
    default:
      return 'generic';
  }
}
