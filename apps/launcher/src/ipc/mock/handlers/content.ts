/** Mock handlers for the content area: search, project details, categories, install. */
import type { ContentSummary, InstallTarget } from '../../types/content';
import type { InstalledContent } from '../../types/instances';
import {
  CATEGORIES_BY_KIND,
  catalog,
  computeCompatibility,
  findProject,
  projectDetail,
  projectVersions,
  searchCatalog,
  type CatalogProject,
  type CompatContext,
} from '../data/content';
import { serversDb } from '../data/servers';
import { startDownload } from '../downloads';
import { daysAgo, emit, fail, latency, uid, type MockHandlers } from '../runtime';
import { addInstanceContent, db, findInstance } from './core';

/** Projects installed on mock servers (the server admin area keeps its own plugin list). */
const serverInstalls = new Map<string, Set<string>>();

function installedOnServer(serverId: string): Set<string> {
  let s = serverInstalls.get(serverId);
  if (!s) {
    s = new Set();
    serverInstalls.set(serverId, s);
  }
  return s;
}

function contextFor(target: InstallTarget | undefined): CompatContext {
  if (!target) return { target: null, installed: new Set() };
  if (target.kind === 'instance' || target.kind === 'world') {
    const inst = findInstance(target.instance_id);
    const installed = new Set(
      (db.content[inst.id] ?? []).map((c) => c.project_id).filter((x): x is string => !!x),
    );
    return { target: target.kind, horizonVersion: inst.horizon_version, installed };
  }
  const srv =
    serversDb.servers.find((s) => s.id === target.server_id) ??
    fail('not_found', `server ${target.server_id}`);
  return {
    target: target.kind,
    horizonVersion: srv.horizon_version,
    serverEngine: srv.engine,
    installed: installedOnServer(srv.id),
  };
}

function installedIn(projectId: string): string[] {
  const ids: string[] = [];
  for (const [instanceId, list] of Object.entries(db.content)) {
    if (list.some((c) => c.project_id === projectId)) ids.push(instanceId);
  }
  for (const [serverId, set] of serverInstalls) if (set.has(projectId)) ids.push(serverId);
  return ids;
}

function toSummary(
  p: CatalogProject,
  compatibility: ContentSummary['compatibility'],
): ContentSummary {
  return {
    id: p.id,
    slug: p.name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
    kind: p.kind,
    source: p.source,
    name: p.name,
    summary: p.summary,
    author: p.author,
    icon_url: null,
    downloads: p.downloads,
    follows: p.follows,
    updated_at: daysAgo(p.updated_days),
    categories: p.categories,
    compatibility,
    installed_in: installedIn(p.id),
    featured: !!p.featured && !p.fair_play_violation,
    shader_format: p.shader_format,
  };
}

function filename(p: CatalogProject, version: string): string {
  const base = p.name.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  switch (p.kind) {
    case 'mod':
      return `${base}-${version}.hmod`;
    case 'shader':
      return p.shader_format === 'horizon'
        ? `${base}-${version}.hshader`
        : `${base}-${version}.zip`;
    case 'plugin':
      return `${base}-${version}.rplugin`;
    default:
      return `${base}-${version}.zip`;
  }
}

/** Required dependencies not yet on the target, recursively. */
function missingDeps(
  p: CatalogProject,
  installed: ReadonlySet<string>,
  acc = new Map<string, CatalogProject>(),
): CatalogProject[] {
  for (const d of p.deps ?? []) {
    if (d.relation !== 'required' || installed.has(d.project_id) || acc.has(d.project_id)) continue;
    const dep = findProject(d.project_id);
    if (!dep) continue;
    acc.set(dep.id, dep);
    missingDeps(dep, installed, acc);
  }
  return [...acc.values()];
}

export const contentHandlers: MockHandlers = {
  content_search: async (q) => {
    await latency(1.4);
    const ctx = contextFor(q.target);
    const { hits, total } = searchCatalog(catalog, q, (p) => computeCompatibility(p, ctx));
    return {
      hits: hits.map((h) => toSummary(h.project, h.compatibility)),
      total,
      offset: q.offset,
      limit: q.limit,
      has_more: q.offset + q.limit < total,
    };
  },

  content_get: async ({ project_id, target }) => {
    await latency(1.2);
    const p = findProject(project_id) ?? fail('not_found', `project ${project_id}`);
    const ctx = contextFor(target);
    const versions = projectVersions(p, (horizon) => computeCompatibility(p, ctx, horizon));
    const detail = projectDetail(p, toSummary(p, computeCompatibility(p, ctx)), versions);
    detail.dependencies = (p.deps ?? []).map((d) => ({
      project_id: d.project_id,
      name: findProject(d.project_id)?.name ?? d.project_id,
      relation: d.relation,
      installed: ctx.installed.has(d.project_id),
    }));
    return detail;
  },

  content_categories: async ({ kind }) => {
    await latency(0.4);
    return (CATEGORIES_BY_KIND[kind] ?? []).map((id) => ({ id, kind }));
  },

  content_install: async ({ project_id, version_id, target, with_dependencies }) => {
    await latency();
    const p = findProject(project_id) ?? fail('not_found', `project ${project_id}`);
    if (p.kind === 'modpack') {
      fail('invalid_argument', 'Modpacks are installed as a new instance (instance_import).');
    }
    const ctx = contextFor(target);
    if (ctx.installed.has(p.id)) fail('conflict', `${p.name} is already installed here.`);
    const versions = projectVersions(p, (h) => computeCompatibility(p, ctx, h));
    const version = versions.find((v) => v.id === version_id) ?? versions[0]!;
    if (version.compatibility.status === 'incompatible') {
      fail('conflict', version.compatibility.detail, { reason: version.compatibility.reason });
    }
    const items: { project: CatalogProject; version: string }[] = [
      { project: p, version: version.version },
    ];
    if (with_dependencies) {
      for (const d of missingDeps(p, ctx.installed)) items.push({ project: d, version: d.version });
    }
    const totalMb = items.reduce((a, i) => a + i.project.size_mb, 0);

    const instanceId =
      target.kind === 'instance' || target.kind === 'world' ? target.instance_id : undefined;
    const serverId =
      target.kind === 'server' || target.kind === 'server_world' ? target.server_id : undefined;

    const apply = () => {
      for (const { project, version: v } of items) {
        if (instanceId) {
          const list = db.content[instanceId] ?? [];
          const item: InstalledContent = {
            id: uid('ic'),
            project_id: project.id,
            kind: project.kind,
            name: project.name,
            version: v,
            filename: filename(project, v),
            enabled: true,
            source: project.source,
            icon_url: null,
            update_available: null,
            compatible: true,
            incompatibility: null,
            order:
              project.kind === 'resourcepack'
                ? list.filter((c) => c.kind === 'resourcepack').length
                : undefined,
            world_id: target.kind === 'world' ? target.world_id : undefined,
          };
          if (target.kind === 'world') {
            const world = (db.worlds[instanceId] ?? []).find((w) => w.id === target.world_id);
            const inst = db.instances.find((i) => i.id === instanceId);
            if (world) {
              if (world.datapack_count === 0 && inst) inst.datapack_world_count += 1;
              world.datapack_count += 1;
            }
          }
          addInstanceContent(instanceId, item);
        } else if (serverId) {
          installedOnServer(serverId).add(project.id);
          const srv = serversDb.servers.find((s) => s.id === serverId);
          if (srv && project.kind === 'plugin') srv.plugin_count += 1;
        }
      }
      if (serverId) emit('servers-changed', structuredClone(serversDb.servers));
    };

    // As the desktop side: the call answers once the install is over, its task named after the
    // project ("Install <id>", which the UI follows for progress meanwhile).
    let task_id = '';
    await new Promise<void>((resolve, reject) => {
      task_id = startDownload({
        kind: 'content',
        label: `Install ${p.id}`,
        instance_id: instanceId,
        server_id: serverId,
        total_bytes: Math.round(totalMb * 1024 ** 2),
        duration_ms: 1600 + 500 * items.length,
        onDone: () => {
          apply();
          resolve();
        },
        onCancel: () => {
          try {
            fail('cancelled', 'The install was cancelled.');
          } catch (e) {
            reject(e as Error);
          }
        },
      });
    });
    return {
      task_id,
      installed: items.map((i) => ({
        project_id: i.project.id,
        name: i.project.name,
        version: i.version,
      })),
    };
  },
};
