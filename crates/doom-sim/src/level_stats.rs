//! Observation-only work maxima for measuring a level run (M3h9c): the largest
//! intercept list of one path traversal and the largest `spechit` list of one
//! position check. The counters are per thread, never read by the simulation and
//! change no state, RNG draw or outcome; work done inside a tic that is later
//! refused is still counted. Every policy updates them.
use std::cell::Cell;

/// Work maxima observed on this thread since the last
/// [`take_level_run_stats`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LevelRunStats {
    /// Most intercepts one `P_PathTraverse` collected (hitscan and aim traces:
    /// lines and things; slide and use traces: lines), against the source's
    /// `MAXINTERCEPTS` 128; an overflowing traversal counts as 129.
    pub max_intercepts: usize,
    /// Most `spechit` entries one `P_CheckPosition` collected, against the
    /// source's `MAXSPECIALCROSS` 8; an overflowing check counts as 9.
    pub max_spechit: usize,
}

std::thread_local! {
    static STATS: Cell<LevelRunStats> = const {
        Cell::new(LevelRunStats { max_intercepts: 0, max_spechit: 0 })
    };
}

/// Return the maxima observed on this thread and reset them to zero.
pub fn take_level_run_stats() -> LevelRunStats {
    STATS.with(|s| s.replace(LevelRunStats::default()))
}

pub(crate) fn observe_intercepts(count: usize) {
    STATS.with(|s| {
        let mut v = s.get();
        v.max_intercepts = v.max_intercepts.max(count);
        s.set(v);
    });
}

pub(crate) fn observe_spechit(count: usize) {
    STATS.with(|s| {
        let mut v = s.get();
        v.max_spechit = v.max_spechit.max(count);
        s.set(v);
    });
}
