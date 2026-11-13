/** Cloud mock: the signed-in account's marketplace library and its own creations (ADR 0155). */
import type { CloudClient } from '../client';
import { CloudError } from '../errors';
import type { LibraryItem, MarketplaceProject } from '../types';
import { catalog, type CatalogProject } from '../../ipc/mock/data/content';
import { daysAgo, latency } from '../../ipc/mock/runtime';

function toProject(p: CatalogProject): MarketplaceProject {
  return {
    id: p.id.replace(/^rustic:/, ''),
    slug: p.id.replace(/^rustic:/, ''),
    kind: p.native ? 'native_mod' : p.kind,
    name: p.name,
    summary: p.summary,
    author: p.author,
    icon_url: null,
    banner_url: p.banner ?? null,
    downloads: p.downloads,
    verified: !!p.verified,
    price: p.price_cents ? { amount_minor: p.price_cents, currency: 'EUR' } : null,
    visibility: 'public',
    review: p.native ? 'approved' : 'not_required',
    taken_down: false,
    updated_at: daysAgo(p.updated_days),
    latest_version: p.version,
  };
}

const byId = (id: string) => catalog.find((p) => p.id === `rustic:${id}`);

const library = new Map<string, LibraryItem>(
  (['sky_islands', 'arena_kit'] as const).map((id, i) => [
    id,
    { project: toProject(byId(id)!), source: 'free', added_at: daysAgo(2 + i * 3) },
  ]),
);

/** Sam's creations: a public modpack and a native mod waiting for review (private meanwhile). */
const mine: MarketplaceProject[] = [
  toProject(byId('cozy_village')!),
  {
    id: 'sam_tools',
    slug: 'sam_tools',
    kind: 'native_mod',
    name: 'Sam’s Build Tools',
    summary: 'Copy, mirror and rotate selections while building. Waiting for review.',
    author: 'Sam',
    icon_url: null,
    banner_url: null,
    downloads: 0,
    verified: false,
    price: null,
    visibility: 'private',
    review: 'pending',
    taken_down: false,
    updated_at: daysAgo(1),
    latest_version: '0.1.0',
  },
];

export const contentMock: Partial<CloudClient> = {
  async contentLibrary() {
    await latency(0.6);
    return structuredClone([...library.values()]);
  },
  async libraryAdd(id) {
    await latency(0.5);
    const p = byId(id);
    if (!p) throw new CloudError('not_found', 'project not found', 404);
    if (p.price_cents) throw new CloudError('paid_content', 'paid projects are not sold yet', 409);
    library.set(id, { project: toProject(p), source: 'free', added_at: new Date().toISOString() });
  },
  async libraryRemove(id) {
    await latency(0.4);
    library.delete(id);
  },
  async myContent() {
    await latency(0.6);
    return structuredClone(mine);
  },
  async reportContent() {
    await latency(0.6);
  },
};
