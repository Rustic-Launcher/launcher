/** Shared mock server state (servers area + server admin area both read/write it). */
import type { Server, ServerAccess } from '../../types/servers';
import type { DomainBinding } from '../../types/dns';
import { daysAgo, minutesAgo } from '../runtime';

export const serversDb: {
  servers: Server[];
  access: Record<string, ServerAccess>;
  domains: Record<string, DomainBinding>;
} = {
  servers: [
    {
      id: 'srv_copains',
      name: 'Les Copains SMP',
      icon: { kind: 'preset', preset: 'grass', color: '#4E9F5B' },
      engine: 'rustic',
      horizon_version: 'v0.2.15',
      horizon_version_label: 'Horizon 0.2.15',
      hosting: { mode: 'local' },
      state: 'running',
      players_online: 4,
      max_players: 20,
      port: 25565,
      motd: 'Bienvenue chez les copains !',
      world_name: 'copains',
      memory_mb: 4096,
      created_at: daysAgo(60),
      last_started_at: minutesAgo(95),
      share_address: 'copains.relay.example.invalid',
      plugin_count: 5,
      folder: 'C:/Users/Sam/AppData/Roaming/RusticLauncher/servers/les-copains-smp',
    },
    {
      id: 'srv_vps',
      name: 'Survie publique',
      icon: { kind: 'preset', preset: 'castle', color: '#D4A24C' },
      engine: 'rustic',
      horizon_version: 'v0.2.15',
      horizon_version_label: 'Horizon 0.2.15',
      hosting: { mode: 'vps', connection_id: 'vps_ovh', host: '203.0.113.84' },
      state: 'running',
      players_online: 17,
      max_players: 60,
      port: 25565,
      motd: 'Survie publique — règles sur /rules',
      world_name: 'world',
      memory_mb: 6144,
      created_at: daysAgo(25),
      last_started_at: daysAgo(3),
      share_address: 'play.samcraft.fr',
      plugin_count: 8,
      folder: '/srv/rustic/survie-publique',
    },
    {
      id: 'srv_creative',
      name: 'Creative test',
      icon: { kind: 'preset', preset: 'gem', color: '#3A8FB7' },
      engine: 'horizon',
      horizon_version: 'v0.2.14',
      horizon_version_label: 'Horizon 0.2.14',
      hosting: { mode: 'local' },
      state: 'stopped',
      players_online: 0,
      max_players: 10,
      port: 25566,
      motd: 'A Horizon server',
      world_name: 'flatland',
      memory_mb: 2048,
      created_at: daysAgo(9),
      last_started_at: daysAgo(4),
      share_address: null,
      plugin_count: 0,
      folder: 'C:/Users/Sam/AppData/Roaming/RusticLauncher/servers/creative-test',
    },
  ],
  access: {},
  domains: {},
};
