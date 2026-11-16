/** Pure helpers of the Java Launchpad (ADR 0172), unit-tested in logic.test.ts. */
import type {
  JavaInstallStep,
  JavaLoader,
  JavaLogLine,
  JavaProfile,
  JavaRunState,
} from '@/ipc/types/java';

/**
 * The official edition icons, loaded at run time from minecraft.net (never shipped with Rustic:
 * no Mojang asset in the repository); a neutral icon stands in when they can't load.
 */
export const OFFICIAL_ICONS = {
  java: 'https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/logos/Java-24px.png',
  bedrock:
    'https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/logos/Bedrock-24px.png',
} as const;

export const LOADERS: JavaLoader[] = ['vanilla', 'fabric', 'quilt', 'forge', 'neoforge'];

/** Display names of the loaders (product names, never translated). */
export const LOADER_NAMES: Record<JavaLoader, string> = {
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
};

/** `Fabric 0.20.1 · 1.21.1`, `Vanilla 26.3`. */
export function profileVersionLabel(
  p: Pick<JavaProfile, 'loader' | 'loader_version' | 'game_version'>,
) {
  if (p.loader === 'vanilla') return `${LOADER_NAMES.vanilla} ${p.game_version}`;
  return p.loader_version
    ? `${LOADER_NAMES[p.loader]} ${p.loader_version} · ${p.game_version}`
    : `${LOADER_NAMES[p.loader]} · ${p.game_version}`;
}

/** The profile the Launchpad shows: the remembered one, else the last played, else the first. */
export function pickProfile(
  profiles: JavaProfile[] | undefined,
  remembered: string | null,
): JavaProfile | undefined {
  if (!profiles?.length) return undefined;
  const r = remembered && profiles.find((p) => p.id === remembered);
  if (r) return r;
  return [...profiles].sort(
    (a, b) => (b.last_played_at ?? b.created_at) - (a.last_played_at ?? a.created_at),
  )[0];
}

export type LaunchAction = 'launch' | 'install_launch' | 'stop' | 'cancel' | 'starting';

/** What the big button does for a profile in a state. */
export function launchAction(
  p: Pick<JavaProfile, 'installed' | 'running'>,
  s: JavaRunState | undefined,
): LaunchAction {
  if (s?.state === 'installing') return 'cancel';
  if (s?.state === 'launching') return 'starting';
  if (s?.state === 'running' || p.running) return 'stop';
  return p.installed ? 'launch' : 'install_launch';
}

/** The i18n key under `java:step.*` for an install step. */
export function stepKey(step: JavaInstallStep | undefined): `step.${JavaInstallStep}` {
  return `step.${step ?? 'metadata'}`;
}

/** The i18n key of a launch failure reason (`java:error.*`), when it has its own text. */
export function errorKey(reason: string | undefined) {
  switch (reason) {
    case 'auth.no_account':
      return 'error.noAccount' as const;
    case 'auth.not_owned':
      return 'error.notOwned' as const;
    case 'auth.reauth_required':
    case 'auth.expired':
      return 'error.reauth' as const;
    case 'java.no_runtime':
    case 'java.no_java':
      return 'error.noJava' as const;
    case 'java.installer':
      return 'error.installer' as const;
    case 'java.no_loader_version':
      return 'error.noLoaderVersion' as const;
    case 'java.network':
    case 'java.download':
      return 'error.network' as const;
    default:
      return null;
  }
}

/** Memory in MB as GB text (`4 GB`, `1.5 GB`). */
export function formatMemory(mb: number): string {
  const gb = mb / 1024;
  return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
}

/** Seconds played as `12 h`, `45 min`. */
export function formatPlayTime(seconds: number): { h: number } | { min: number } {
  if (seconds >= 3600) return { h: Math.round(seconds / 3600) };
  return { min: Math.max(0, Math.round(seconds / 60)) };
}

/** The memory slider's top: the system's RAM minus 2 GB, between 2 and 32 GB, in 512 MB steps. */
export function memoryMax(totalBytes: number | undefined): number {
  const mb = totalBytes ? Math.floor(totalBytes / 1024 / 1024) - 2048 : 16_384;
  return Math.max(2048, Math.min(32_768, Math.floor(mb / 512) * 512));
}

/** JVM arguments typed in a text field (one per space; quotes keep spaces). */
export function parseJvmArgs(text: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) out.push(m[1] ?? m[2] ?? '');
  return out.filter((a) => a.length > 0);
}

export function formatJvmArgs(args: string[]): string {
  return args.map((a) => (/\s/.test(a) ? `"${a}"` : a)).join(' ');
}

/** The log lines a filter shows. */
export function filterLog(
  lines: JavaLogLine[],
  level: 'all' | 'warn' | 'error',
  query: string,
): JavaLogLine[] {
  const q = query.trim().toLowerCase();
  return lines.filter(
    (l) =>
      (level === 'all' || l.level === 'error' || (level === 'warn' && l.level === 'warn')) &&
      (!q || l.text.toLowerCase().includes(q)),
  );
}
