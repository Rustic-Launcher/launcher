import { Avatar, useSkinHead, type AvatarProps } from '@rustic/ui';
import type { Account } from '@/ipc/types/auth';

/** A game account's avatar: the face of its skin, or its initials while loading or without one. */
export function AccountAvatar({
  account,
  ...rest
}: { account: Account } & Omit<AvatarProps, 'name' | 'src'>) {
  const head = useSkinHead(account.skin?.url);
  return <Avatar name={account.username} src={head} {...rest} />;
}
