/** Content queries, install tracking and mutations. */
import { toast } from '@rustic/ui';
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { create } from 'zustand';
import { cloud } from '@/cloud/client';
import type { ContentReportReason } from '@/cloud/types';
import { invoke, type IpcFailure } from '@/ipc';
import type {
  ContentKind,
  ContentSearchQuery,
  ContentSource,
  InstallTarget,
} from '@/ipc/types/content';
import { qk } from '@/queries/core';
import { useActivity } from '@/stores/activity';

export const PAGE_SIZE = 12;

export const ck = {
  all: ['content'] as const,
  search: (q: Omit<ContentSearchQuery, 'offset' | 'limit'>) => ['content', 'search', q] as const,
  detail: (id: string, target?: InstallTarget) =>
    ['content', 'detail', id, target ?? null] as const,
  categories: (kind: ContentKind) => ['content', 'categories', kind] as const,
  list: (q: Omit<ContentSearchQuery, 'offset'>) => ['content', 'list', q] as const,
  servers: ['servers'] as const,
  worlds: (instanceId: string) => qk.instanceWorlds(instanceId),
  library: ['content-library'] as const,
  mine: ['content-mine'] as const,
};

/** One page of results (Discover's hero and rows). */
export function useContentList(q: Omit<ContentSearchQuery, 'offset'>, enabled = true) {
  return useQuery({
    queryKey: ck.list(q),
    queryFn: () => invoke('content_search', { ...q, offset: 0 }),
    enabled,
    staleTime: 60_000,
  });
}

/** The signed-in Rustic account's library (`GET /v1/me/library`). */
export function useContentLibrary(enabled: boolean) {
  return useQuery({
    queryKey: ck.library,
    queryFn: async () => (await cloud()).contentLibrary(),
    enabled,
    retry: false,
  });
}

/** The projects the signed-in account publishes (`GET /v1/me/content`). */
export function useMyContent(enabled: boolean) {
  return useQuery({
    queryKey: ck.mine,
    queryFn: async () => (await cloud()).myContent(),
    enabled,
    retry: false,
  });
}

export function useLibraryMutations() {
  const { t } = useTranslation('content');
  const qc = useQueryClient();
  const refresh = () => void qc.invalidateQueries({ queryKey: ck.library });
  const onError = (e: Error) =>
    toast({ tone: 'error', title: t('library.failed'), description: e.message });
  return {
    add: useMutation({
      mutationFn: async (v: { id: string; name: string }) => (await cloud()).libraryAdd(v.id),
      onSuccess: (_r, v) => {
        toast({ tone: 'success', title: t('library.added', { name: v.name }) });
        refresh();
      },
      onError,
    }),
    remove: useMutation({
      mutationFn: async (v: { id: string; name: string }) => (await cloud()).libraryRemove(v.id),
      onSuccess: (_r, v) => {
        toast({ tone: 'success', title: t('library.removed', { name: v.name }) });
        refresh();
      },
      onError,
    }),
  };
}

/** Reports a Rustic project to the moderators (signed in). */
export function useReportContent() {
  const { t } = useTranslation('content');
  return useMutation({
    mutationFn: async (v: { id: string; reason: ContentReportReason }) =>
      (await cloud()).reportContent(v.id, v.reason),
    onSuccess: () =>
      toast({ tone: 'success', title: t('report.sent'), description: t('report.sentBody') }),
    onError: (e: Error) =>
      toast({ tone: 'error', title: t('report.failed'), description: e.message }),
  });
}

export function useContentSearch(q: Omit<ContentSearchQuery, 'offset' | 'limit'>, enabled = true) {
  return useInfiniteQuery({
    queryKey: ck.search(q),
    queryFn: ({ pageParam }) =>
      invoke('content_search', { ...q, offset: pageParam, limit: PAGE_SIZE }),
    initialPageParam: 0,
    getNextPageParam: (last) =>
      (last.has_more ?? last.offset + last.limit < last.total)
        ? last.offset + last.limit
        : undefined,
    enabled,
    placeholderData: (prev) => prev,
  });
}

export function useContentDetail(projectId: string, target?: InstallTarget) {
  return useQuery({
    queryKey: ck.detail(projectId, target),
    queryFn: () => invoke('content_get', { project_id: projectId, target }),
    placeholderData: (prev) => prev,
  });
}

export function useCategories(kind: ContentKind) {
  return useQuery({
    queryKey: ck.categories(kind),
    queryFn: () => invoke('content_categories', { kind }),
    staleTime: 10 * 60_000,
  });
}

export function useServers() {
  return useQuery({ queryKey: ck.servers, queryFn: () => invoke('servers_list') });
}

export function useWorlds(instanceId: string | undefined) {
  return useQuery({
    queryKey: ck.worlds(instanceId ?? ''),
    queryFn: () => invoke('instance_worlds_list', { instance_id: instanceId! }),
    enabled: !!instanceId,
  });
}

/**
 * In-flight installs keyed by `<projectId>|<targetId>`. `content_install` answers once the install
 * is over (its task has already finished by then), so an install is "in flight" exactly while the
 * call is: set when it starts, cleared when it settles, whatever its outcome.
 */
interface InstallState {
  pending: Record<string, true>;
  start: (key: string) => void;
  end: (key: string) => void;
}

export const useInstalls = create<InstallState>()((set) => ({
  pending: {},
  start: (key) => set((s) => ({ pending: { ...s.pending, [key]: true } })),
  end: (key) =>
    set((s) => ({
      pending: Object.fromEntries(Object.entries(s.pending).filter(([k]) => k !== key)),
    })),
}));

export const installKey = (projectId: string, targetId: string) => `${projectId}|${targetId}`;

/** The backend's label for an install's task (`content_install`: "Install <project id>"). */
const installTaskLabel = (projectId: string) => `Install ${projectId}`;

/** Live progress (0..1 or null) of an install, or undefined when none is running. */
export function useInstallProgress(projectId: string, targetId: string | undefined) {
  const pending = useInstalls((s) =>
    targetId ? !!s.pending[installKey(projectId, targetId)] : false,
  );
  const task = useActivity((s) =>
    pending
      ? Object.values(s.downloads).find(
          (t) =>
            t.kind === 'content' &&
            t.label === installTaskLabel(projectId) &&
            (t.state === 'running' || t.state === 'queued'),
        )
      : undefined,
  );
  if (!pending) return undefined;
  if (!task || !task.total_bytes) return null;
  return task.received_bytes / task.total_bytes;
}

export function useInstall() {
  const { t } = useTranslation('content');
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (v: {
      projectId: string;
      name: string;
      targetId: string;
      targetName: string;
      target: InstallTarget;
      versionId?: string;
    }) => {
      const key = installKey(v.projectId, v.targetId);
      useInstalls.getState().start(key);
      try {
        return await invoke('content_install', {
          project_id: v.projectId,
          version_id: v.versionId,
          target: v.target,
          with_dependencies: true,
        });
      } finally {
        useInstalls.getState().end(key);
      }
    },
    onSuccess: (res, v) => {
      const deps = res.installed.length - 1;
      toast({
        tone: 'success',
        title: t('install.started', { name: v.name, target: v.targetName }),
        description: deps > 0 ? t('install.withDeps', { count: deps }) : undefined,
      });
      void qc.invalidateQueries({ queryKey: ck.all });
      void qc.invalidateQueries({ queryKey: qk.instances });
      void qc.invalidateQueries({ queryKey: ck.servers });
    },
    onError: (e: IpcFailure, v) => {
      toast({
        tone: 'error',
        title: t('install.failed', { name: v.name }),
        description: e.message,
      });
    },
  });
}

/** Modpacks become a new instance. */
export function useCreateFromModpack() {
  const { t } = useTranslation('content');
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { projectId: string; name: string }) =>
      invoke('instance_import', { kind: 'modpack', project_id: v.projectId, name: v.name }),
    onSuccess: (_res, v) => {
      toast({
        tone: 'success',
        title: t('modpack.created', { name: v.name }),
        description: t('modpack.createdHint'),
      });
      void qc.invalidateQueries({ queryKey: qk.instances });
    },
    onError: (e: IpcFailure, v) => {
      toast({
        tone: 'error',
        title: t('install.failed', { name: v.name }),
        description: e.message,
      });
    },
  });
}

export const ALL_SOURCES: ContentSource[] = ['horizon', 'modrinth', 'rustic'];

/** `rustic:<id>` → `<id>` (the catalogue's own id), else null. */
export function rusticId(projectId: string): string | null {
  return projectId.startsWith('rustic:') ? projectId.slice('rustic:'.length) : null;
}
export const KINDS: ContentKind[] = [
  'mod',
  'modpack',
  'resourcepack',
  'shader',
  'datapack',
  'plugin',
];
