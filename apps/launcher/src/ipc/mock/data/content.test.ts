import { describe, expect, it } from 'vitest';
import type { ContentSearchQuery } from '../../types/content';
import {
  catalog,
  computeCompatibility,
  findProject,
  fold,
  bump,
  matchesQuery,
  projectVersions,
  searchCatalog,
  type CompatContext,
} from './content';

const inst = (horizonVersion: string, installed: string[] = []): CompatContext => ({
  target: 'instance',
  horizonVersion,
  installed: new Set(installed),
});
const get = (id: string) => findProject(id)!;

describe('computeCompatibility', () => {
  it('marks mods with a build for the target version', () => {
    expect(computeCompatibility(get('p_minimap'), inst('v0.2.15'))).toEqual({
      status: 'compatible',
      builds_on_horizon: true,
    });
  });
  it('rejects mods without a build for the target version', () => {
    const c = computeCompatibility(get('p_torches'), inst('v0.2.15'));
    expect(c).toMatchObject({ status: 'incompatible', reason: 'horizon_version' });
  });
  it('rejects Java-only projects', () => {
    expect(computeCompatibility(get('p_create'), inst('v0.2.15'))).toMatchObject({
      reason: 'java_only',
    });
  });
  it('flags fair-play violations whatever the target', () => {
    expect(computeCompatibility(get('p_xray'), inst('v0.2.15'))).toMatchObject({
      reason: 'not_fair_play',
    });
  });
  it('detects conflicts with installed content', () => {
    expect(computeCompatibility(get('p_photon'), inst('v0.2.15', ['p_sodiumlike']))).toMatchObject({
      reason: 'conflict',
    });
    expect(computeCompatibility(get('p_photon'), inst('v0.2.15'))).toMatchObject({
      status: 'compatible',
    });
  });
  it('only allows plugins on Rustic Server servers', () => {
    const pl = get('pl_essentials');
    expect(computeCompatibility(pl, inst('v0.2.15'))).toMatchObject({
      reason: 'needs_rustic_server',
    });
    const horizonServer: CompatContext = {
      target: 'server',
      horizonVersion: 'v0.2.15',
      serverEngine: 'horizon',
      installed: new Set(),
    };
    expect(computeCompatibility(pl, horizonServer)).toMatchObject({
      reason: 'needs_rustic_server',
    });
    expect(computeCompatibility(pl, { ...horizonServer, serverEngine: 'rustic' })).toMatchObject({
      status: 'compatible',
    });
  });
  it('treats packs as format-level compatible, but not on servers', () => {
    expect(computeCompatibility(get('p_rp_ui'), inst('v0.2.13'))).toEqual({
      status: 'compatible',
      builds_on_horizon: false,
    });
    expect(
      computeCompatibility(get('p_sh_golden'), {
        target: 'server',
        serverEngine: 'rustic',
        horizonVersion: 'v0.2.15',
        installed: new Set(),
      }),
    ).toMatchObject({ reason: 'side' });
  });
  it('rejects unsupported shader layouts', () => {
    expect(computeCompatibility(get('p_sh_legacy'), inst('v0.2.15'))).toMatchObject({
      reason: 'unsupported_format',
    });
  });
  it('is unknown for mods without a target', () => {
    expect(computeCompatibility(get('p_minimap'), { target: null, installed: new Set() })).toEqual({
      status: 'unknown',
    });
  });
  it('judges modpacks against the current Horizon release', () => {
    expect(
      computeCompatibility(get('pack_essentials'), { target: null, installed: new Set() }),
    ).toMatchObject({ status: 'compatible' });
    expect(
      computeCompatibility(get('pack_cozy'), { target: null, installed: new Set() }),
    ).toMatchObject({ reason: 'horizon_version' });
  });
});

const query = (q: Partial<ContentSearchQuery>): ContentSearchQuery => ({
  query: '',
  kind: 'mod',
  sources: [],
  compatible_only: true,
  sort: 'relevance',
  offset: 0,
  limit: 100,
  ...q,
});

describe('searchCatalog', () => {
  const compat = (ctx: CompatContext) => (p: (typeof catalog)[number]) =>
    computeCompatibility(p, ctx);

  it('hides incompatible and fair-play violations when compatible only is on', () => {
    const { hits } = searchCatalog(catalog, query({}), compat(inst('v0.2.15')));
    const ids = hits.map((h) => h.project.id);
    expect(ids).not.toContain('p_xray');
    expect(ids).not.toContain('p_create');
    expect(ids).toContain('p_minimap');
  });
  it('shows incompatible results last for fair-play violations when the filter is off', () => {
    const { hits } = searchCatalog(
      catalog,
      query({ compatible_only: false }),
      compat(inst('v0.2.15')),
    );
    expect(hits.at(-1)?.project.id).toBe('p_xray');
  });
  it('filters by source, category and accent-insensitive text', () => {
    const rustic = searchCatalog(catalog, query({ sources: ['rustic'] }), compat(inst('v0.2.15')));
    expect(rustic.hits.every((h) => h.project.source === 'rustic')).toBe(true);
    const perf = searchCatalog(
      catalog,
      query({ category: 'performance' }),
      compat(inst('v0.2.15')),
    );
    expect(perf.hits.every((h) => h.project.categories.includes('performance'))).toBe(true);
    const fr = searchCatalog(catalog, query({ query: 'boites shulker' }), compat(inst('v0.2.15')));
    expect(fr.hits[0]?.project.id).toBe('p_shulker');
  });
  it('ranks exact name matches first and paginates', () => {
    const { hits, total } = searchCatalog(
      catalog,
      query({ query: 'minimap' }),
      compat(inst('v0.2.15')),
    );
    expect(hits[0]?.project.id).toBe('p_minimap');
    const page = searchCatalog(catalog, query({ limit: 3, offset: 3 }), compat(inst('v0.2.15')));
    expect(page.hits).toHaveLength(3);
    expect(page.total).toBeGreaterThan(6);
    expect(total).toBeGreaterThanOrEqual(1);
  });
  it('sorts by downloads', () => {
    const { hits } = searchCatalog(catalog, query({ sort: 'downloads' }), compat(inst('v0.2.15')));
    const d = hits.map((h) => h.project.downloads);
    expect([...d].sort((a, b) => b - a)).toEqual(d);
  });
});

describe('helpers', () => {
  it('steps back through versions with a borrow', () => {
    expect(bump('1.6.0', 1)).toBe('1.5.9');
    expect(bump('2.2.0', 2)).toBe('2.1.8');
    expect(bump('5.2', 3)).toBe('4.9');
    expect(bump('1.0.0', 0)).toBe('1.0.0');
  });
  it('folds accents', () => {
    expect(fold('Boîte À Outils')).toBe('boite a outils');
    expect(matchesQuery(get('p_rp_dark'), 'nuit profonde')).toBe(true);
  });
  it('builds a version history whose older releases target older Horizon versions', () => {
    const versions = projectVersions(get('p_sodiumlike'), () => ({ status: 'unknown' }));
    expect(versions[0]?.horizon_versions).toEqual(['v0.2.14', 'v0.2.15']);
    expect(versions[1]?.horizon_versions).toEqual(['v0.2.13', 'v0.2.14']);
    expect(versions).toHaveLength(4);
  });
  it('has about forty projects across every kind', () => {
    expect(catalog.length).toBeGreaterThanOrEqual(40);
    for (const k of ['mod', 'modpack', 'resourcepack', 'shader', 'datapack', 'plugin']) {
      expect(catalog.some((p) => p.kind === k)).toBe(true);
    }
  });
});
