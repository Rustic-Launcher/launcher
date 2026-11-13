import { Avatar, useSkinHead, type AvatarProps } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@/ipc';

/** Any player's avatar by game profile: the face of their skin, or their initials without one. */
export function PlayerAvatar({
  uuid,
  name,
  ...rest
}: { uuid: string; name: string } & Omit<AvatarProps, 'name' | 'src'>) {
  const skin = useQuery({
    queryKey: ['player-skin', uuid],
    queryFn: () => invoke('player_skin_url', { uuid }),
    staleTime: 60 * 60_000,
    retry: false,
  });
  const head = useSkinHead(skin.data ?? undefined);
  return <Avatar name={name} src={head} {...rest} />;
}
