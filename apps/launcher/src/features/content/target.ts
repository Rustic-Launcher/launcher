/** Install target encoding for the URL (`instance:<id>` | `server:<id>`) and helpers. */
import type { ContentKind, ContentSummary, InstallTarget } from '@/ipc/types/content';
import type { Instance } from '@/ipc/types/instances';
import type { Server } from '@/ipc/types/servers';

export type TargetRef = { kind: 'instance'; id: string } | { kind: 'server'; id: string };

export function parseTarget(s: string | undefined | null): TargetRef | null {
  if (!s) return null;
  const i = s.indexOf(':');
  if (i <= 0) return null;
  const kind = s.slice(0, i);
  const id = s.slice(i + 1);
  if (!id || (kind !== 'instance' && kind !== 'server')) return null;
  return { kind, id };
}

export function formatTarget(t: TargetRef): string {
  return `${t.kind}:${t.id}`;
}

/** Which targets a kind of content can go to. */
export function targetKindsFor(kind: ContentKind): TargetRef['kind'][] {
  switch (kind) {
    case 'plugin':
      return ['server'];
    case 'datapack':
      return ['instance', 'server'];
    case 'modpack':
      return [];
    default:
      return ['instance'];
  }
}

/** Servers able to receive this kind (plugins: Rustic Server engine only). */
export function eligibleServers(kind: ContentKind, servers: Server[]): Server[] {
  if (kind === 'plugin') return servers.filter((s) => s.engine === 'rustic');
  if (kind === 'datapack') return servers;
  return [];
}

/**
 * Resolves the effective target: the one in the URL when valid for this kind, else the active
 * instance (or the first eligible server for plugins).
 */
export function resolveTarget(
  kind: ContentKind,
  requested: TargetRef | null,
  instances: Instance[],
  servers: Server[],
  activeInstanceId: string | undefined,
): TargetRef | null {
  const kinds = targetKindsFor(kind);
  if (kinds.length === 0) return null;
  const valid = (t: TargetRef | null): t is TargetRef =>
    !!t &&
    kinds.includes(t.kind) &&
    (t.kind === 'instance'
      ? instances.some((i) => i.id === t.id)
      : eligibleServers(kind, servers).some((s) => s.id === t.id));
  if (valid(requested)) return requested;
  if (kinds.includes('instance')) {
    const id = activeInstanceId ?? instances[0]?.id;
    if (id) return { kind: 'instance', id };
  }
  const srv = eligibleServers(kind, servers)[0];
  return srv ? { kind: 'server', id: srv.id } : null;
}

/** IPC install target, given the chosen world for datapacks. */
export function toInstallTarget(t: TargetRef, kind: ContentKind, worldId?: string): InstallTarget {
  if (t.kind === 'server') {
    return kind === 'datapack'
      ? { kind: 'server_world', server_id: t.id }
      : { kind: 'server', server_id: t.id };
  }
  if (kind === 'datapack' && worldId)
    return { kind: 'world', instance_id: t.id, world_id: worldId };
  return { kind: 'instance', instance_id: t.id };
}

/** Target used for compatibility computation (worlds share their instance's rules). */
export function toQueryTarget(t: TargetRef | null, kind: ContentKind): InstallTarget | undefined {
  if (!t) return undefined;
  return toInstallTarget(t, kind);
}

export function isInstalledOn(
  item: Pick<ContentSummary, 'installed_in'>,
  t: TargetRef | null,
): boolean {
  return !!t && item.installed_in.includes(t.id);
}

/** Deterministic hue for generated icons/covers. */
export function hueOf(id: string): number {
  let h = 7;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) % 360;
  return h;
}
