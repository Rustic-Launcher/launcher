#!/usr/bin/env node
// Generates launcher/docs/IPC.md from src/ipc/commands.ts + src/ipc/types/*.ts + src/cloud/client.ts.
// Usage: pnpm gen:ipc-doc   (CI can run it with --check to fail when IPC.md is stale)
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const app = path.resolve(here, '..');
const out = path.resolve(app, '../../docs/IPC.md');
const read = (p) => readFileSync(path.join(app, p), 'utf8').replace(/\r\n/g, '\n');

const commandsSrc = read('src/ipc/commands.ts');

function block(src, startMarker) {
  const start = src.indexOf(startMarker);
  if (start < 0) throw new Error(`marker not found: ${startMarker}`);
  let depth = 0;
  for (let i = src.indexOf('{', start); i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(src.indexOf('{', start) + 1, i);
    }
  }
  throw new Error('unbalanced');
}

/** Parses `name: [Args, Result];` entries (possibly multi-line) with `// ---- section ----` headers. */
function parseCommands(body) {
  const sections = [];
  let current = { title: 'misc', items: [] };
  let doc = '';
  let buf = '';
  for (const raw of body.split('\n')) {
    const line = raw.trim();
    const sec = line.match(/^\/\/ ----\s*(.+?)\s*----$/);
    if (sec) {
      if (current.items.length) sections.push(current);
      current = { title: sec[1], items: [] };
      continue;
    }
    const jsdoc = line.match(/^\/\*\*\s*(.+?)\s*\*\/$/);
    if (jsdoc) {
      doc = jsdoc[1];
      continue;
    }
    if (!line || line.startsWith('//')) continue;
    buf += (buf ? ' ' : '') + line;
    if (!buf.endsWith(';')) continue;
    const m = buf.match(/^([a-z0-9_]+):\s*\[(.*)\];$/);
    buf = '';
    if (!m) continue;
    // split "[Args, Result]" at the top-level comma
    const inner = m[2];
    let depth = 0;
    let cut = -1;
    for (let i = 0; i < inner.length; i++) {
      const c = inner[i];
      if ('<{[('.includes(c)) depth++;
      else if ('>}])'.includes(c)) depth--;
      else if (c === ',' && depth === 0) {
        cut = i;
        break;
      }
    }
    const args = inner.slice(0, cut).trim();
    const result = inner.slice(cut + 1).trim();
    current.items.push({ name: m[1], args, result, doc });
    doc = '';
  }
  if (current.items.length) sections.push(current);
  return sections;
}

function parseEvents(body) {
  const items = [];
  for (const raw of body.split('\n')) {
    const m = raw.trim().match(/^'([a-z-]+)':\s*(.+);$/);
    if (m) items.push({ name: m[1], payload: m[2] });
  }
  return items;
}

const aliases = [...commandsSrc.matchAll(/^type (\w+) = (.+);$/gm)].map(
  (m) => `type ${m[1]} = ${m[2]};`,
);
const sections = parseCommands(block(commandsSrc, 'export interface IpcCommands'));
const events = parseEvents(block(commandsSrc, 'export interface IpcEvents'));
const count = sections.reduce((n, s) => n + s.items.length, 0);

const typeFiles = readdirSync(path.join(app, 'src/ipc/types'))
  .filter((f) => f.endsWith('.ts'))
  .sort();
const cloudClient = block(read('src/cloud/client.ts'), 'export interface CloudClient');
const cloudTypes = read('src/cloud/types.ts');

const esc = (s) => s.replace(/\|/g, '\\|');
const rust = (n) =>
  n
    .split('_')
    .map((w) => w[0].toUpperCase() + w.slice(1))
    .join('');

let md = `# Rustic Launcher IPC (UI ↔ Tauri)

Owners: frontend (TypeScript client + browser mock, ADR 0024) and desktop (Rust implementation in
\`apps/launcher/src-tauri\`). **Generated** from \`apps/launcher/src/ipc/commands.ts\` and
\`src/ipc/types/*.ts\` by \`pnpm --filter @rustic/launcher gen:ipc-doc\`: edit the TypeScript, then regenerate.

${count} commands, ${events.length} events.

## Conventions

- **Commands** are \`snake_case\`. Each takes **one argument named \`args\`** (a struct whose fields are
  \`snake_case\`) or no argument at all. The TS client calls \`invoke(name, { args })\`. Rust side:

  \`\`\`rust
  #[derive(serde::Deserialize)]
  pub struct ServerCommandArgs { pub server_id: String, pub line: String }

  #[tauri::command]
  async fn server_command(args: ServerCommandArgs, state: tauri::State<'_, App>) -> Result<(), IpcError> { … }
  \`\`\`

  Argument struct names in this document: \`<CommandInPascalCase>Args\` (e.g. \`ServerCommandArgs\`). Results and
  payloads serialise with serde defaults (\`snake_case\` fields, \`#[serde(tag = "kind")]\` /
  \`#[serde(tag = "mode")]\` / \`#[serde(tag = "status")]\` for the tagged unions shown in the types below,
  \`#[serde(rename_all = "snake_case")]\` for string enums; event names and the \`in-game\` presence are kebab-case).
- **Errors**: every command returns \`Result<T, IpcError>\`; \`IpcError = { code, message, details? }\` with \`code\` one of
  \`not_found | invalid_argument | conflict | cancelled | network | unauthorized | ownership_required |
  parental_block | unavailable | internal\`. Messages are developer-facing; the UI localises by \`code\`.
- **Events** are \`kebab-case\`, emitted with \`app.emit(name, payload)\` to all windows.
- **Long work** (downloads, installs, preparation, VPS install, DNS propagation) returns immediately with a \`task_id\`
  and reports through events (\`download-progress\`, \`downloads-changed\`, \`prepare-progress\`, \`hosting-install\`,
  \`dns-status\`); \`download_cancel\` cancels by task id.
- **Secrets** (Porkbun keys, SSH passwords/passphrases) travel only inside IPC (never on a command line) and are
  stored in the OS keychain by the backend; they are never returned to the UI (only masked labels).
- Dates are ISO-8601 UTC strings; sizes in bytes; memory in MiB; money in minor units with an ISO currency.
- The browser mock (\`src/ipc/mock\`) implements every command below with realistic data so the UI runs without Tauri.

Helper aliases used in the tables:

\`\`\`ts
${aliases.join('\n')}
\`\`\`

## Commands
`;

for (const s of sections) {
  md += `\n### ${s.title.charAt(0).toUpperCase()}${s.title.slice(1)}\n\n| Command | \`args\` (Rust: \`<Name>Args\`) | Result |\n|---|---|---|\n`;
  for (const c of s.items) {
    const args = c.args === 'void' ? '—' : `\`${esc(c.args)}\``;
    const note = c.doc ? `<br>${esc(c.doc)}` : '';
    md += `| \`${c.name}\`${note} | ${args} | \`${esc(c.result)}\` |\n`;
    void rust;
  }
}

md += `\n## Events\n\n| Event | Payload |\n|---|---|\n`;
for (const e of events) md += `| \`${e.name}\` | \`${esc(e.payload)}\` |\n`;

md += `\n## Types\n\nVerbatim from \`apps/launcher/src/ipc/types/\`. Rust mirrors use the same field names.\n`;
for (const f of typeFiles) {
  md += `\n### ${f}\n\n\`\`\`ts\n${read(`src/ipc/types/${f}`).trim()}\n\`\`\`\n`;
}

md += `
## Appendix: Rustic Cloud API expected by the UI (not IPC)

Pages use the \`CloudClient\` interface (\`apps/launcher/src/cloud/client.ts\`, ADR 0027). Default build: mock. With
\`VITE_CLOUD_URL\` set, \`src/cloud/http.ts\` implements it over \`services/api/openapi.json\` (types generated by
\`pnpm --filter @rustic/launcher gen:cloud\`, openapi-typescript; requests via openapi-fetch; bearer token from the
\`auth_rustic_access_token\` command). Gaps between this interface and the API are tracked in HANDOFF.md.

\`\`\`ts
export interface CloudClient {${cloudClient}}
\`\`\`

UI-side shapes (\`apps/launcher/src/cloud/types.ts\`):

\`\`\`ts
${cloudTypes.trim()}
\`\`\`
`;

if (process.argv.includes('--check')) {
  const current = readFileSync(out, 'utf8').replace(/\r\n/g, '\n');
  if (current !== md) {
    console.error('docs/IPC.md is stale: run pnpm --filter @rustic/launcher gen:ipc-doc');
    process.exit(1);
  }
  console.log('IPC.md up to date');
} else {
  writeFileSync(out, md);
  console.log(
    `wrote ${path.relative(process.cwd(), out)} (${count} commands, ${events.length} events)`,
  );
}
