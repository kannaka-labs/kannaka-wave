//! Surprise as a dual-timescale differentiator: ADR-0040's cerebellar novelty
//! operator, ported from kannaka-memory `src/novelty.rs` (a8725c7), for the
//! world organ's salience path (ADR-0002) and E-004's arm S.
//!
//! Two leaky integrators of the same drive `u = g·r`, a fast `a_i` and a much
//! slower `a_e`, and their difference. The two branches share one gain, so a
//! constant drive at any level cancels exactly (`H(0) = 0`): the baseline is
//! rejected, not thresholded. That is what stops a channel that is always
//! noisy from being remembered as permanently interesting.
//!
//! **One change from the original, and it is the sign.** kannaka-memory feeds
//! the operator *familiarity* (the top recall resonance), where surprise is a
//! **drop**, and computes `slow − fast`. E-004 and ADR-0002 feed it a
//! prediction **error**, `d(Ẑ, Z)`, where surprise is a **rise**, and ask for
//! `fast − slow`. The same operator "unchanged" would fire when the world
//! model was unusually *right*. So the meaning of the drive is a required
//! argument, [`Drive`], and each sign has its own tests. Everything else (time
//! constants, gain, threshold homeostasis, per-context bank) is the original.

use std::collections::HashMap;

/// Fast smoothing `a_i = dt/tau_I` (`tau_I ≈ 2` observations).
pub const DEFAULT_A_I: f32 = 0.5;
/// Slow smoothing `a_e = dt/tau_E` (`tau_E ≈ 50` observations).
pub const DEFAULT_A_E: f32 = 0.02;
/// Matched gain, shared by both branches so `H(0) = 0`.
pub const DEFAULT_G: f32 = 1.0;
/// Threshold-homeostasis rate (`≪ a_e ≪ a_i`).
pub const DEFAULT_BETA: f32 = 0.01;
/// Threshold sensitivity: `theta = mean + k·std` of the score.
pub const DEFAULT_K: f32 = 3.0;

/// What the drive measures, which fixes which direction is surprising.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// Familiarity (a recall's top similarity): surprise is a **drop** below
    /// the learned baseline. kannaka-memory's use; `raw = slow − fast`.
    Familiarity,
    /// Prediction error (the world organ's `d(Ẑ, Z)`): surprise is a **rise**
    /// above the learned baseline. E-004's use; `raw = fast − slow`.
    Error,
}

/// One observation's outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Novelty {
    /// Directional surprise, `max(raw, 0)`. Zero is routine.
    pub score: f32,
    /// Signed differentiation; negative is *less* surprising than routine.
    pub raw: f32,
    /// The adaptive boundary `theta = mean + k·std` of `score`.
    pub theta: f32,
    /// `score > theta`.
    pub novel: bool,
}

/// One channel: a fast EMA, a slow EMA and a self-tuning threshold. O(1).
#[derive(Debug, Clone, Copy)]
pub struct DualTimescaleNovelty {
    drive: Drive,
    fast: f32,
    slow: f32,
    g: f32,
    a_i: f32,
    a_e: f32,
    warm: bool,
    m: f32,
    v: f32,
    beta: f32,
    k: f32,
}

impl DualTimescaleNovelty {
    /// A channel with the default time constants.
    pub fn new(drive: Drive) -> Self {
        Self::with_params(
            drive,
            DEFAULT_A_I,
            DEFAULT_A_E,
            DEFAULT_G,
            DEFAULT_BETA,
            DEFAULT_K,
        )
    }

    /// A channel with explicit parameters. `a_i` must exceed `a_e`; `g` is
    /// shared by both branches.
    pub fn with_params(drive: Drive, a_i: f32, a_e: f32, g: f32, beta: f32, k: f32) -> Self {
        Self {
            drive,
            fast: 0.0,
            slow: 0.0,
            g,
            a_i,
            a_e,
            warm: false,
            m: 0.0,
            v: 0.0,
            beta,
            k,
        }
    }

    /// Observe one drive value. The first observation seeds both branches, so
    /// there is no cold-start spike.
    pub fn observe(&mut self, r: f32) -> Novelty {
        let u = self.g * r;
        if !self.warm {
            self.fast = u;
            self.slow = u;
            self.warm = true;
        }
        self.fast += self.a_i * (u - self.fast);
        self.slow += self.a_e * (u - self.slow);
        let raw = match self.drive {
            Drive::Familiarity => self.slow - self.fast,
            Drive::Error => self.fast - self.slow,
        };
        let score = raw.max(0.0);
        let d = score - self.m;
        self.m += self.beta * d;
        self.v += self.beta * (d * d - self.v);
        let theta = self.m + self.k * self.v.max(0.0).sqrt();
        Novelty {
            score,
            raw,
            theta,
            novel: score > theta,
        }
    }

    /// The learned baseline (the slow branch).
    pub fn baseline(&self) -> f32 {
        self.slow
    }
}

/// A bank of channels keyed by context, so surprise on one channel (a
/// metrics watcher posting every minute) does not blur into another.
#[derive(Debug, Clone)]
pub struct NoveltyDetector {
    drive: Drive,
    map: HashMap<String, DualTimescaleNovelty>,
}

impl NoveltyDetector {
    /// A bank whose channels all read this kind of drive.
    pub fn new(drive: Drive) -> Self {
        Self {
            drive,
            map: HashMap::new(),
        }
    }

    /// Observe a drive within a context.
    pub fn observe(&mut self, context: &str, drive: f32) -> Novelty {
        let kind = self.drive;
        self.map
            .entry(context.to_string())
            .or_insert_with(|| DualTimescaleNovelty::new(kind))
            .observe(drive)
    }

    /// Distinct contexts tracked.
    pub fn contexts(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold(ch: &mut DualTimescaleNovelty, r: f32, n: usize) -> Novelty {
        let mut last = ch.observe(r);
        for _ in 1..n {
            last = ch.observe(r);
        }
        last
    }

    /// E-004's guard, "a constant stream gives surprise 0", held to exactly
    /// zero rather than a tolerance: the first observation seeds both
    /// branches at `u`, and `x += a·(u − x)` leaves `x = u` unchanged.
    #[test]
    fn a_constant_stream_is_exactly_zero_for_both_drives() {
        for drive in [Drive::Familiarity, Drive::Error] {
            for level in [0.0f32, 0.3, 0.9, 1.7, -0.4, 1234.5] {
                let mut ch = DualTimescaleNovelty::new(drive);
                for _ in 0..500 {
                    let n = ch.observe(level);
                    assert_eq!(n.score, 0.0, "{drive:?} at {level}");
                    assert_eq!(n.raw, 0.0, "{drive:?} at {level}");
                    assert!(!n.novel);
                }
            }
        }
    }

    /// The original's baseline-rejection test, after a step: the residue of a
    /// 0.6 step is `0.6 · (1 − a_e)^n`, about 3e-6 at n = 600 (≈ 12 slow time
    /// constants; at 400 it is still 1.8e-4).
    #[test]
    fn a_new_constant_is_relearned_as_baseline() {
        for drive in [Drive::Familiarity, Drive::Error] {
            let mut ch = DualTimescaleNovelty::new(drive);
            hold(&mut ch, 0.2, 300);
            let last = hold(&mut ch, 0.8, 600);
            assert!(last.score < 1e-4, "{drive:?}: {}", last.score);
        }
    }

    #[test]
    fn familiarity_fires_on_a_drop_and_never_on_a_rise() {
        let mut ch = DualTimescaleNovelty::new(Drive::Familiarity);
        hold(&mut ch, 0.9, 300);
        let n = ch.observe(0.1);
        assert!(n.novel && n.raw > 0.0, "{n:?}");
        let mut up = DualTimescaleNovelty::new(Drive::Familiarity);
        hold(&mut up, 0.1, 300);
        assert_eq!(up.observe(0.9).score, 0.0, "more familiar is never novel");
    }

    #[test]
    fn error_fires_on_a_rise_and_never_on_a_drop() {
        let mut ch = DualTimescaleNovelty::new(Drive::Error);
        hold(&mut ch, 0.1, 300);
        let n = ch.observe(0.9);
        assert!(n.novel && n.raw > 0.0, "the world model was wrong: {n:?}");
        let mut down = DualTimescaleNovelty::new(Drive::Error);
        hold(&mut down, 0.9, 300);
        assert_eq!(
            down.observe(0.1).score,
            0.0,
            "a prediction better than usual is not surprising"
        );
    }

    /// Why the sign is an argument: the operator ported unchanged (the
    /// familiarity sign) and fed an error scores the world model's best
    /// prediction as surprising and its worst as routine.
    #[test]
    fn the_familiarity_sign_on_an_error_drive_is_inverted() {
        let mut wrong = DualTimescaleNovelty::new(Drive::Familiarity);
        hold(&mut wrong, 0.5, 300);
        let big_miss = wrong.observe(0.95).score;
        let mut wrong2 = DualTimescaleNovelty::new(Drive::Familiarity);
        hold(&mut wrong2, 0.5, 300);
        let near_hit = wrong2.observe(0.05).score;
        assert_eq!(big_miss, 0.0);
        assert!(near_hit > 0.0);
    }

    #[test]
    fn a_sustained_error_habituates() {
        let mut ch = DualTimescaleNovelty::new(Drive::Error);
        hold(&mut ch, 0.1, 300);
        assert!(ch.observe(0.9).novel, "precondition");
        let settled = hold(&mut ch, 0.9, 200);
        assert!(
            !settled.novel && settled.score <= settled.theta,
            "{settled:?}"
        );
    }

    #[test]
    fn a_one_off_peaks_above_a_sustained_step() {
        let mut a = DualTimescaleNovelty::new(Drive::Error);
        hold(&mut a, 0.1, 300);
        let peak = a.observe(0.9).score;
        let mut b = DualTimescaleNovelty::new(Drive::Error);
        hold(&mut b, 0.1, 300);
        let steady = hold(&mut b, 0.9, 200).score;
        assert!(peak > steady + 1e-3, "{peak} vs {steady}");
    }

    /// ADR-0040's collapse check, kept: equal timescales make the operator
    /// identically zero, so a test that only checked "no false alarms" would
    /// pass on a dead operator.
    #[test]
    fn equal_timescales_are_a_dead_operator() {
        let mut dead = DualTimescaleNovelty::with_params(Drive::Error, 0.3, 0.3, 1.0, 0.01, 3.0);
        hold(&mut dead, 0.1, 300);
        assert_eq!(dead.observe(0.9).score, 0.0);
    }

    #[test]
    fn contexts_are_isolated() {
        let mut det = NoveltyDetector::new(Drive::Error);
        for _ in 0..300 {
            det.observe("metrics", 0.1);
            det.observe("mail", 0.1);
        }
        assert!(det.observe("metrics", 0.9).novel);
        let mail = det.observe("mail", 0.1);
        assert!(!mail.novel && mail.score == 0.0);
        assert_eq!(det.contexts(), 2);
    }
}
