# 0055. System information

- Status: accepted (desktop, 2026-11-10)

## Context
The UI shows memory and CPU for RAM sliders and server limits.

## Decision

`sysinfo` 0.37 (MIT) with only the `system` feature, refreshed on demand.

## Consequences

Small and portable.

## Alternatives

Platform calls per OS.
