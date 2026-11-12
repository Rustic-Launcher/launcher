import type { IsoDate } from './app';

export interface ServerPing {
  online: boolean;
  latency_ms: number | null;
  motd: string;
  players_online: number;
  players_max: number;
  version: string;
}

/** An entry of the game's multiplayer list (servers.dat), shared by all instances. */
export interface MultiplayerEntry {
  id: string;
  name: string;
  address: string;
  favorite: boolean;
  last_joined_at: IsoDate | null;
}
