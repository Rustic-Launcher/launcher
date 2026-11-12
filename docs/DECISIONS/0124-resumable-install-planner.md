# 0124. Resumable install plans with reverse rollback

- Status: accepted (hosting, 2026-11-10)

## Decision

`plan::run_plan` runs `Step`s (`is_done` checked on the target, `apply`, `rollback`). Done steps are skipped, so a
re-run resumes; the truth lives on the VPS, not in the launcher. On failure, the steps applied in this run (the
failed one included) are rolled back in reverse order; rollback errors are collected and reported, never hidden.
Every transition emits a `ProgressEvent {operation, step, title, index, total, state, detail, bytes}`
(`model::ui_step` maps step ids to the frontend's coarser `InstallStepId`). Uninstall is a plan of idempotent steps
without rollback.

## Consequences

Re-running an install is always safe; a failed install leaves no running unit or open firewall port behind. Cost:
a few extra `test` commands per run.

## Alternatives

A local journal of applied steps (diverges from the VPS when the user changes things by hand).
