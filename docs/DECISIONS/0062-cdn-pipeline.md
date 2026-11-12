# 0062. Cosmetics CDN pipeline: validate, optimize, content-address, never rewrite

- Status: accepted (content, 2026-11-10)
- Owners affected: cloud (publishes the output folder, serves the manifest)

## Decision

`rustic-cdn-pipeline build <input> --out <dir> [--quantize] [--generated TS]` (`launcher/services/cdn-pipeline`): for
each package (folder or zip): read (zips past 8 MiB unpacked and unsafe names refused), validate, optimize (unused
files and empty unnamed parts dropped, PNG re-encoded without ancillary chunks, optionally coordinates snapped to
1/16 px and alpha made cutout), validate again, then write:

```
cosmetics/<id>/<version>/<sha256>.json   self-contained bundle (canonical JSON, keys sorted)
horizon/<id>/<sha256>.json               Horizon item, when representable
textures/<sha256>.png                    the texture, for previews and thumbnails
manifest.json                            latest version per id, every version with hashes, sizes, stats
```

Everything but `manifest.json` is immutable (`Cache-Control: public, max-age=31536000, immutable`). The same id and
version with other bytes is refused ("bump the version"); re-running on the same input changes no byte. Rejected
packages are reported (JSON) and written nowhere; the exit code is non-zero. `validate` and `schema` subcommands too.

## Consequences

Clients verify a bundle by its address (the companion checks SHA-256 before use). The cloud uploads the folder and
serves `manifest.json` with a short TTL.

## Crates

`clap` 4 (MIT OR Apache-2.0), `zip` 4 (MIT), `sha2` 0.10 and `hex` 0.4 (MIT OR Apache-2.0), `tempfile` 3 (tests).
