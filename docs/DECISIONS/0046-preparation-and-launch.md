# 0046. Preparation and launch through Horizon's Launch API

- Status: accepted (desktop, 2026-11-10)

## Context
horizon/docs/api/launch.md, launch-reference.md, launch-guide.md: ask `--launch-api` first, pass only listed
options (others exit 2), session JSON on stdin, `--progress json`, `--check` before every start.

## Decision

- Both programs are probed (`launch_api` 0.1.x only). `prepare_args` and `launch_args` refuse an unlisted
  option before anything starts; the player's extra game arguments are checked the same way, and options the
  launcher sets (`--session-*`, `--resources`, `--game-dir`, `--home`) are refused there.
- Preparation: `--prepare --minecraft-dir|... --java-home --output <data>/prepared/<build> --complete
  --progress json`; progress lines parsed (unknown keys ignored, fractions never decrease); a Java of
  `java_major` found from configured homes, JAVA_HOME, PATH, the usual folders and the official launcher's
  runtimes (`release` files, not started). The server jar is downloaded (sha1-checked) only with a
  `VerifiedOwner`.
- Launch: `--resources --game-dir <instance>/game --home <instance>/game [--width --height --fullscreen
  --quickPlay*] --session-stdin`; the session JSON is written to stdin and stdin closed. Logs stream; exit
  0/1/2/101 mapped; the crash report from the last stderr line, else the newest file in `crash-reports/`;
  playtime added at exit. `HORIZON_LAUNCHER_EXE` is not set (RISKS R-12).

## Consequences

RAM settings are kept and shown but not passed: Horizon is native and the Launch API has no memory option.

## Alternatives

`--session-file` (touches the disk); passing options blindly.
