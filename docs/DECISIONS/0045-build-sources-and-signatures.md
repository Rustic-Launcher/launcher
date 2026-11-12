# 0045. Horizon build sources: GitHub and a signed channel index

- Status: accepted (desktop, 2026-11-10)

## Context
User decision (2026-11-10): the launcher installs a prebuilt Rustic edition (ADR 0006), and plain Horizon
releases stay installable. Horizon's releases carry `SHA256SUMS` and Ed25519 signatures over
`horizon release <tag>\n` + SHA256SUMS; current Horizon launchers require a Nightly's `SHA256SUMS.v2.sig`,
signed for `nightly-<date>-<commit>` read strictly from its title (crates/horizon-release-sign).
launch-guide.md still describes the channel signature: the code wins (RISKS).

## Decision

- `[builds]` in launcher.toml lists sources: `kind = "github"` (repo, asset prefix) or `kind = "channel"`
  (a JSON index), each with a signature namespace and base64 Ed25519 keys; Horizon's key is built in
  (copied from Horizon's release-keys.txt at a84f637d).
- Channel index format 1: `{"format": 1, "builds": [{"version", "title"?, "published_at", "prerelease",
  "notes", "base_url", "assets": [{"name", "size"}]}]}`; `<base_url>SHA256SUMS` and `SHA256SUMS.sig`, signed
  for the version under `rustic release`. Rustic's CI publishes it (TODO_HUMAN D3).
- Install: fetch and verify the signed list (refuse unsigned, other namespace/version/key), download with the
  signed SHA-256, unpack (zip `enclosed_name`, tar `unpack_in`), keep the single top folder, check
  `horizon-client`, write `rustic-build.json`. Verification uses `ring` (already in the tree).

## Consequences

Releases from before Horizon signed (legacy-releases.txt) are not offered. The Rustic edition source has no
key until our CI key exists: installs from it are refused until then.

## Alternatives

Depending on `horizon-release-sign` by path (outside this branch, pulls Horizon's workspace).
