//! The step planner behind installs, updates and uninstalls (ADR 0124).
//!
//! A plan is an ordered list of steps. Each step can tell whether it is already done (so a re-run
//! after a failure, a lost connection or a closed launcher resumes instead of starting over: the
//! state lives on the VPS itself, not in the launcher), apply itself, and undo itself. When a step
//! fails, the steps **this run** applied are rolled back in reverse order; steps found already done
//! belong to an earlier successful run and are left alone. Every transition is a progress event.

use crate::error::{HostingError, Result};
use crate::model::{ProgressEvent, ProgressSink, StepState};
use async_trait::async_trait;

/// What a step reports while it runs.
pub struct StepProgress<'a> {
    sink: &'a dyn ProgressSink,
    operation: &'a str,
    step: &'a str,
    title: &'a str,
    index: usize,
    total: usize,
}

impl StepProgress<'_> {
    fn event(&self, state: StepState, detail: Option<String>, bytes: Option<(u64, u64)>) {
        self.sink.emit(ProgressEvent {
            operation: self.operation.to_owned(),
            step: self.step.to_owned(),
            title: self.title.to_owned(),
            index: self.index,
            total: self.total,
            state,
            detail,
            bytes,
        });
    }

    /// A line of detail under the running step.
    pub fn detail(&self, text: impl Into<String>) {
        self.event(StepState::Running, Some(text.into()), None);
    }

    /// Transfer progress.
    pub fn bytes(&self, done: u64, total: u64) {
        self.event(StepState::Running, None, Some((done, total)));
    }

    /// A warning the user should read (shown even when the step succeeds).
    pub fn warn(&self, text: impl Into<String>) {
        self.event(StepState::Warning, Some(text.into()), None);
    }
}

#[async_trait]
pub trait Step<C: Send + Sync>: Send + Sync {
    /// Stable id (`probe`, `install-horizon`...), for the UI and the logs.
    fn id(&self) -> &'static str;
    /// What the user reads.
    fn title(&self) -> String;
    /// Already done (checked on the target itself)? Done steps are skipped.
    async fn is_done(&self, ctx: &C) -> Result<bool>;
    async fn apply(&self, ctx: &C, progress: &StepProgress<'_>) -> Result<()>;
    /// Undo `apply`. Must be safe to call when `apply` failed halfway.
    async fn rollback(&self, _ctx: &C) -> Result<()> {
        Ok(())
    }
}

/// How each step went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanReport {
    pub applied: Vec<&'static str>,
    pub skipped: Vec<&'static str>,
}

/// Run `steps` in order; on failure roll back what this run applied, then return
/// [`HostingError::StepFailed`].
pub async fn run_plan<C: Send + Sync>(
    operation: &str,
    steps: &[Box<dyn Step<C>>],
    ctx: &C,
    sink: &dyn ProgressSink,
) -> Result<PlanReport> {
    let total = steps.len();
    let mut applied: Vec<usize> = Vec::new();
    let mut skipped = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let title = step.title();
        let progress = StepProgress {
            sink,
            operation,
            step: step.id(),
            title: &title,
            index: index + 1,
            total,
        };
        progress.event(StepState::Running, None, None);
        let outcome = match step.is_done(ctx).await {
            Ok(true) => {
                progress.event(StepState::Skipped, Some("already done".into()), None);
                skipped.push(step.id());
                continue;
            }
            Ok(false) => step.apply(ctx, &progress).await,
            Err(e) => Err(e),
        };
        match outcome {
            Ok(()) => {
                progress.event(StepState::Done, None, None);
                applied.push(index);
            }
            Err(error) => {
                progress.event(StepState::Failed, Some(error.to_string()), None);
                // The failed step may have done part of its work: undo it too.
                applied.push(index);
                let rollback_errors = rollback(operation, steps, &applied, ctx, sink).await;
                return Err(HostingError::StepFailed {
                    step: title,
                    source: Box::new(error),
                    rollback_errors,
                });
            }
        }
    }
    Ok(PlanReport {
        applied: applied.iter().map(|&i| steps[i].id()).collect(),
        skipped,
    })
}

async fn rollback<C: Send + Sync>(
    operation: &str,
    steps: &[Box<dyn Step<C>>],
    applied: &[usize],
    ctx: &C,
    sink: &dyn ProgressSink,
) -> Vec<String> {
    let mut errors = Vec::new();
    for &index in applied.iter().rev() {
        let step = &steps[index];
        let title = step.title();
        let progress = StepProgress {
            sink,
            operation,
            step: step.id(),
            title: &title,
            index: index + 1,
            total: steps.len(),
        };
        progress.event(StepState::RollingBack, None, None);
        match step.rollback(ctx).await {
            Ok(()) => progress.event(StepState::RolledBack, None, None),
            Err(e) => {
                progress.event(StepState::Warning, Some(format!("undo failed: {e}")), None);
                errors.push(format!("{title}: {e}"));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::Mutex;

    /// A pretend machine: a set of things that exist, a log of what happened.
    #[derive(Default)]
    struct World {
        state: Mutex<BTreeSet<&'static str>>,
        log: Mutex<Vec<String>>,
        fail_on: Mutex<Option<&'static str>>,
        fail_rollback_of: Mutex<Option<&'static str>>,
    }

    struct Make(&'static str);

    #[async_trait]
    impl Step<World> for Make {
        fn id(&self) -> &'static str {
            self.0
        }
        fn title(&self) -> String {
            format!("make {}", self.0)
        }
        async fn is_done(&self, w: &World) -> Result<bool> {
            Ok(w.state.lock().unwrap().contains(self.0))
        }
        async fn apply(&self, w: &World, p: &StepProgress<'_>) -> Result<()> {
            w.log.lock().unwrap().push(format!("apply {}", self.0));
            p.detail("working");
            if *w.fail_on.lock().unwrap() == Some(self.0) {
                return Err(HostingError::Timeout(self.0.into()));
            }
            w.state.lock().unwrap().insert(self.0);
            Ok(())
        }
        async fn rollback(&self, w: &World) -> Result<()> {
            w.log.lock().unwrap().push(format!("undo {}", self.0));
            if *w.fail_rollback_of.lock().unwrap() == Some(self.0) {
                return Err(HostingError::Ssh("lost".into()));
            }
            w.state.lock().unwrap().remove(self.0);
            Ok(())
        }
    }

    fn plan() -> Vec<Box<dyn Step<World>>> {
        vec![
            Box::new(Make("a")),
            Box::new(Make("b")),
            Box::new(Make("c")),
        ]
    }

    #[tokio::test]
    async fn runs_in_order_and_is_idempotent() {
        let w = World::default();
        let events = Mutex::new(Vec::new());
        let sink = |e: ProgressEvent| events.lock().unwrap().push((e.step, e.state));
        let r = run_plan("install", &plan(), &w, &sink).await.unwrap();
        assert_eq!(r.applied, vec!["a", "b", "c"]);
        assert!(r.skipped.is_empty());
        // Second run: nothing to do.
        let r2 = run_plan("install", &plan(), &w, &sink).await.unwrap();
        assert!(r2.applied.is_empty());
        assert_eq!(r2.skipped, vec!["a", "b", "c"]);
        assert_eq!(
            *w.log.lock().unwrap(),
            vec!["apply a", "apply b", "apply c"]
        );
        let ev = events.lock().unwrap();
        assert!(ev.contains(&("a".into(), StepState::Done)));
        assert!(ev.contains(&("c".into(), StepState::Skipped)));
    }

    #[tokio::test]
    async fn failure_rolls_back_this_runs_steps_in_reverse() {
        let w = World::default();
        // "a" exists from an earlier run: it must not be undone.
        w.state.lock().unwrap().insert("a");
        *w.fail_on.lock().unwrap() = Some("c");
        let err = run_plan("install", &plan(), &w, &NoSink).await.unwrap_err();
        match &err {
            HostingError::StepFailed {
                step,
                rollback_errors,
                ..
            } => {
                assert_eq!(step, "make c");
                assert!(rollback_errors.is_empty());
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            *w.log.lock().unwrap(),
            vec!["apply b", "apply c", "undo c", "undo b"]
        );
        assert_eq!(
            w.state.lock().unwrap().iter().copied().collect::<Vec<_>>(),
            vec!["a"]
        );
        // Resume after fixing the cause: only b and c run.
        *w.fail_on.lock().unwrap() = None;
        let r = run_plan("install", &plan(), &w, &NoSink).await.unwrap();
        assert_eq!(r.skipped, vec!["a"]);
        assert_eq!(r.applied, vec!["b", "c"]);
    }

    #[tokio::test]
    async fn rollback_errors_are_reported_and_do_not_stop_the_rest() {
        let w = World::default();
        *w.fail_on.lock().unwrap() = Some("c");
        *w.fail_rollback_of.lock().unwrap() = Some("b");
        let err = run_plan("install", &plan(), &w, &NoSink).await.unwrap_err();
        let HostingError::StepFailed {
            rollback_errors, ..
        } = &err
        else {
            panic!("{err:?}")
        };
        assert_eq!(rollback_errors.len(), 1);
        assert!(rollback_errors[0].starts_with("make b"));
        assert_eq!(
            *w.log.lock().unwrap(),
            vec![
                "apply a", "apply b", "apply c", "undo c", "undo b", "undo a"
            ]
        );
        assert!(err.to_string().contains("rollback incomplete"));
    }

    struct NoSink;
    impl ProgressSink for NoSink {
        fn emit(&self, _: ProgressEvent) {}
    }
}
