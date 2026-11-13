//! What a build reports while it runs: a phase, an overall fraction and a line for the player.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Toolchain,
    Source,
    Tool,
    Mixins,
    Compile,
    Install,
    Cleanup,
    Done,
}

impl Phase {
    /// The share of the whole build each phase stands for, as `(start, end)` of the bar.
    pub fn span(self) -> (f64, f64) {
        match self {
            Self::Toolchain => (0.0, 0.02),
            Self::Source => (0.02, 0.08),
            Self::Tool => (0.08, 0.12),
            Self::Mixins => (0.12, 0.16),
            Self::Compile => (0.16, 0.96),
            Self::Install => (0.96, 0.99),
            Self::Cleanup => (0.99, 1.0),
            Self::Done => (1.0, 1.0),
        }
    }

    /// `inner` (0..1 within the phase) as a fraction of the whole build.
    pub fn overall(self, inner: f64) -> f64 {
        let (a, b) = self.span();
        a + (b - a) * inner.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    /// 0..1 over the whole build.
    pub fraction: f64,
    /// What is happening, in English (the UI shows its own text per phase and this as detail).
    pub message: String,
    /// While compiling: crates done and in all, as Cargo counts them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<(u32, u32)>,
}

impl Progress {
    pub fn new(phase: Phase, inner: f64, message: impl Into<String>) -> Self {
        Self {
            phase,
            fraction: phase.overall(inner),
            message: message.into(),
            units: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_cover_the_bar_in_order() {
        let phases = [
            Phase::Toolchain,
            Phase::Source,
            Phase::Tool,
            Phase::Mixins,
            Phase::Compile,
            Phase::Install,
            Phase::Cleanup,
        ];
        let mut end = 0.0;
        for p in phases {
            let (a, b) = p.span();
            assert!((a - end).abs() < 1e-9, "{p:?} starts where the last ended");
            assert!(b > a);
            end = b;
        }
        assert!((end - 1.0).abs() < 1e-9);
        assert!((Phase::Compile.overall(0.5) - 0.56).abs() < 1e-9);
        assert_eq!(Phase::Compile.overall(7.0), 0.96);
    }
}
