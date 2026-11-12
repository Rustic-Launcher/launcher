# 0065. `.mrpack` import keeps what runs; Rustic packs list sources instead of copying them

- Status: accepted (content, 2026-11-10)

## Decision

- `.mrpack`: resource packs, shader packs (partly) and data packs (into `new-world-datapacks/`) are imported from
  Modrinth's allowed hosts with SHA-512 and SHA-1 checked. Java mod jars are reported "needs port" and handed to
  Horizon's `horizon-port-pack port` when the player has it: Rustic does not reimplement that carry (its 26.3 upgrade
  tables and Java-oracle registrations are Horizon's work). Configs, scripts, `options.txt` and server-only files are
  skipped with their reason. One report line per file.
- Rustic pack (`.rusticpack`, `launcher/docs/CONTENT.md`): `rustic-pack.json` lists each installed item by source,
  version, address and hashes; `overrides/` carries only settings and the player's own files when asked. Import goes
  through the installer (every check).

## Consequences

A pack never redistributes third-party files. Importing needs the network for listed content.

## Alternatives

Copying files into packs (licences); exporting `.mrpack` (cannot express Horizon registry content).
