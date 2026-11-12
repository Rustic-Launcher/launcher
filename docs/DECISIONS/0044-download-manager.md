# 0044. Download manager

- Status: accepted (desktop, 2026-11-10)

## Context
Builds, server jars and content are large; networks drop; the same file is often needed twice.

## Decision

`DownloadManager`: a semaphore-bounded queue; each download resumable with `Range: bytes=N-` on a `.part` file
kept across retries and restarts (206 appends, 200 restarts, 416 starts over); checked by size and
SHA-1/SHA-256/SHA-512 (`sha1`, `sha2`, MIT/Apache); verified files go to a content-addressed cache
`<cache>/<algo>/<xx>/<hex>` (opt-out for one-off archives); a resumed file that fails its hash is fetched once
from scratch; retries with backoff on network errors, 5xx, 408 and 429; cancellation per download, by id, or
all; `DownloadEvent`s on a broadcast channel with global totals.

## Consequences

Tested against wiremock with a range-honouring responder. The content crate adapts it (`CoreDownloader`).

## Alternatives

`trauma`/`downloader` crates (no content addressing, less control); one connection per file with no cache.
