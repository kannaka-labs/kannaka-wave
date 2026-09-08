//! The chiral number system, `10000.00001`, as a type.
//!
//! ADR-0021 (2026-03-22) described a wavefront's magnitude as digits on both sides
//! of a mirror plane, like a decimal point, and gave `ChiralScale` as pseudo-code.
//! It was never implemented: the production medium carries two flat vectors and
//! an energy scalar per hemisphere. This is the first real one.
//!
//! Two rules, from the ADR:
//!
//! 1. **Scale jumps are bilateral.** The number of magnitude positions is the same
//!    on both sides, always. Gaining or losing a position happens to both at once.
//!    This is the structural mirror.
//! 2. **Values within a position grow independently.** The analytical (left) side
//!    grows with use; the holistic (right) side grows with consolidation. Their
//!    ratio is a readable fact about the memory: thought-about, or felt.
//!
//! The `.00001` of full depth is not a value here. ADR-0024 relocated it: the
//! irrational remainder is a property of the whole field (the δ-invariant, the
//! spectral leakage), measured elsewhere. What this type carries is the rational
//! part, honestly.

use core::fmt;

/// Lower bound on positions. Two is ADR-0021's "minimal — reflex": its table
/// counts `10.01` as two positions, one digit-pair each side of the mirror.
pub const MIN_POSITIONS: u8 = 2;

/// Upper bound on positions. Five is ADR-0021's "full depth — integrated".
pub const MAX_POSITIONS: u8 = 5;

/// A value within a position is kept in `[0, 100)`. Values are clamped, not
/// wrapped: a side that outgrows its position is a *scale-up request*, which the
/// caller decides bilaterally.
pub const POSITION_CAP: f32 = 100.0;

/// The largest value a side may hold: two decimal digits below the cap, which is
/// the precision of the ADR's notation (`85.47`). Not `POSITION_CAP - EPSILON`:
/// f32 cannot represent that near 100, it rounds back to the cap, and the first
/// version of this file had a test fail for exactly that reason.
pub const POSITION_TOP: f32 = 99.99;

/// Which hand a value belongs to. Labels follow ADR-0024, not ADR-0021.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    /// Sequential, focal, discriminative. Grows with use.
    Analytical,
    /// Parallel, diffuse, pattern-completing. Grows with consolidation.
    Holistic,
}

/// The magnitude structure of one wavefront.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChiralScale {
    positions: u8,
    analytical: f32,
    holistic: f32,
}

/// Why an operation was refused. Every refusal names its rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleError {
    /// Would exceed [`MAX_POSITIONS`].
    TooDeep,
    /// Would drop below one position. A memory with no magnitude is not a memory.
    TooShallow,
    /// A non-finite or negative value was offered.
    NotAMagnitude,
}

impl ChiralScale {
    /// A fresh perception: minimal depth, hot on the analytical side, a trace on
    /// the holistic side. ADR-0021's `10.01`.
    pub fn fresh() -> Self {
        Self {
            positions: MIN_POSITIONS,
            analytical: 10.0,
            holistic: 1.0,
        }
    }

    /// Construct with explicit values. Values are clamped into the position;
    /// the caller asked for a magnitude, not a scale change.
    pub fn new(positions: u8, analytical: f32, holistic: f32) -> Result<Self, ScaleError> {
        if positions < MIN_POSITIONS {
            return Err(ScaleError::TooShallow);
        }
        if positions > MAX_POSITIONS {
            return Err(ScaleError::TooDeep);
        }
        if !analytical.is_finite() || !holistic.is_finite() || analytical < 0.0 || holistic < 0.0 {
            return Err(ScaleError::NotAMagnitude);
        }
        Ok(Self {
            positions,
            analytical: analytical.min(POSITION_TOP),
            holistic: holistic.min(POSITION_TOP),
        })
    }

    /// Number of magnitude positions. Always the same on both sides: that is the
    /// invariant, and there is deliberately no way to ask for one side's count.
    pub fn positions(&self) -> u8 {
        self.positions
    }

    /// The value held on one side, in `[0, POSITION_CAP)`.
    pub fn value(&self, hand: Hand) -> f32 {
        match hand {
            Hand::Analytical => self.analytical,
            Hand::Holistic => self.holistic,
        }
    }

    /// Grow one side independently (rule 2). Returns `true` if the side hit its
    /// cap, which is the signal to consider a bilateral scale-up. It never scales
    /// on its own: one hand cannot change the number of positions.
    pub fn grow(&mut self, hand: Hand, by: f32) -> Result<bool, ScaleError> {
        if !by.is_finite() {
            return Err(ScaleError::NotAMagnitude);
        }
        let slot = match hand {
            Hand::Analytical => &mut self.analytical,
            Hand::Holistic => &mut self.holistic,
        };
        let next = (*slot + by).max(0.0);
        let capped = next >= POSITION_CAP;
        *slot = next.min(POSITION_TOP);
        Ok(capped)
    }

    /// Scale up: add a magnitude position to **both** sides (rule 1). Each side's
    /// value is carried into the new, larger position, so what was `85.47` reads
    /// `8.5|4.7` against the new cap. Nothing is invented: the new slot's tenth
    /// starts as the old value.
    pub fn scale_up(&mut self) -> Result<(), ScaleError> {
        if self.positions >= MAX_POSITIONS {
            return Err(ScaleError::TooDeep);
        }
        self.positions += 1;
        self.analytical /= 10.0;
        self.holistic /= 10.0;
        Ok(())
    }

    /// Scale down: remove a position from **both** sides. Each side's value is
    /// multiplied into the smaller position and clamped. The information above
    /// the cap is what ADR-0021 said the Fano fold would carry; here it is lost,
    /// and the return value says how much, per side, so the caller can decide
    /// whether that was acceptable.
    pub fn scale_down(&mut self) -> Result<(f32, f32), ScaleError> {
        if self.positions <= MIN_POSITIONS {
            return Err(ScaleError::TooShallow);
        }
        self.positions -= 1;
        let a = self.analytical * 10.0;
        let h = self.holistic * 10.0;
        let lost_a = (a - POSITION_TOP).max(0.0);
        let lost_h = (h - POSITION_TOP).max(0.0);
        self.analytical = a.min(POSITION_TOP);
        self.holistic = h.min(POSITION_TOP);
        Ok((lost_a, lost_h))
    }

    /// The asymmetry ratio: analytical over holistic. Above 1 the memory is more
    /// thought-about than felt; below 1, the reverse. A holistic value of zero
    /// yields `f32::INFINITY`, which is the honest answer for "never consolidated".
    pub fn asymmetry(&self) -> f32 {
        if self.holistic == 0.0 {
            return f32::INFINITY;
        }
        self.analytical / self.holistic
    }

    /// The scalar magnitude a flat store would have carried: both sides, weighted
    /// by depth. Exists so E-001's vector arm can be given the same number.
    pub fn magnitude(&self) -> f32 {
        let depth = 10f32.powi(self.positions as i32 - MIN_POSITIONS as i32);
        (self.analytical + self.holistic) * depth
    }
}

impl Default for ChiralScale {
    fn default() -> Self {
        Self::fresh()
    }
}

/// Renders in ADR-0021's notation: the analytical side left of the point, the
/// holistic side right of it, `positions` digits each. Two positions is `10.01`;
/// five is `10000.00001`, the number the ADR named.
impl fmt::Display for ChiralScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.positions as usize;
        let scale = 10f32.powi(self.positions as i32 - MIN_POSITIONS as i32);
        let left = (self.analytical * scale).round() as u64;
        let right = (self.holistic * scale).round() as u64;
        write!(f, "{left:0width$}.{right:0width$}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_perception_is_ten_point_oh_one() {
        let s = ChiralScale::fresh();
        assert_eq!(s.positions(), MIN_POSITIONS);
        assert_eq!(s.to_string(), "10.01");
        assert!(
            s.asymmetry() > 1.0,
            "fresh memories are thought-about, not felt"
        );
    }

    #[test]
    fn scale_jumps_are_bilateral_and_there_is_no_other_way() {
        let mut s = ChiralScale::new(2, 85.0, 47.0).unwrap();
        s.scale_up().unwrap();
        assert_eq!(s.positions(), 3);
        // Both sides moved into the larger position; neither was invented.
        assert!((s.value(Hand::Analytical) - 8.5).abs() < 1e-5);
        assert!((s.value(Hand::Holistic) - 4.7).abs() < 1e-5);
        // The ratio survived the jump: rule 1 changes structure, not meaning.
        assert!((s.asymmetry() - 85.0 / 47.0).abs() < 1e-5);
    }

    #[test]
    fn values_grow_independently_and_growth_never_changes_positions() {
        let mut s = ChiralScale::fresh();
        for _ in 0..30 {
            s.grow(Hand::Analytical, 2.0).unwrap();
        }
        assert_eq!(
            s.positions(),
            MIN_POSITIONS,
            "one hand cannot change the number of positions"
        );
        assert!(s.value(Hand::Analytical) > s.value(Hand::Holistic));
        s.grow(Hand::Holistic, 60.0).unwrap();
        assert!(s.asymmetry() < 1.2, "consolidation caught up");
    }

    #[test]
    fn hitting_the_cap_requests_a_scale_up_but_does_not_perform_one() {
        let mut s = ChiralScale::new(2, 95.0, 5.0).unwrap();
        let capped = s.grow(Hand::Analytical, 20.0).unwrap();
        assert!(capped);
        assert_eq!(s.positions(), 2);
        assert!(s.value(Hand::Analytical) < POSITION_CAP);
    }

    #[test]
    fn scale_down_reports_exactly_what_it_lost_per_side() {
        let mut s = ChiralScale::new(3, 12.0, 3.0).unwrap(); // 120.030
        let (lost_a, lost_h) = s.scale_down().unwrap();
        assert_eq!(s.positions(), 2);
        assert!(lost_a > 0.0, "120 does not fit in a one-position side");
        assert_eq!(lost_h, 0.0, "30 does");
        assert!(s.value(Hand::Analytical) < POSITION_CAP);
        assert!((s.value(Hand::Holistic) - 30.0).abs() < 1e-4);
    }

    #[test]
    fn depth_is_bounded_both_ways_and_refusals_are_named() {
        let mut s = ChiralScale::new(MAX_POSITIONS, 1.0, 1.0).unwrap();
        assert_eq!(s.scale_up(), Err(ScaleError::TooDeep));
        let mut f = ChiralScale::fresh();
        assert_eq!(f.scale_down(), Err(ScaleError::TooShallow));
        assert_eq!(ChiralScale::new(0, 1.0, 1.0), Err(ScaleError::TooShallow));
        assert_eq!(
            ChiralScale::new(1, 1.0, 1.0),
            Err(ScaleError::TooShallow),
            "the ADR's table starts at two"
        );
        assert_eq!(
            ChiralScale::new(2, f32::NAN, 1.0),
            Err(ScaleError::NotAMagnitude)
        );
        assert_eq!(
            ChiralScale::new(2, -1.0, 1.0),
            Err(ScaleError::NotAMagnitude)
        );
    }

    #[test]
    fn full_depth_renders_as_the_number_the_adr_named() {
        let s = ChiralScale::new(5, 10.0, 0.001).unwrap();
        assert_eq!(s.to_string(), "10000.00001");
        // The last digit is the rational trace of the holistic side, not the
        // irrational remainder; that one is not representable here on purpose.
        assert!(s.asymmetry() > 9_000.0);
    }

    #[test]
    fn never_consolidated_is_infinite_asymmetry_not_a_panic() {
        let s = ChiralScale::new(2, 5.0, 0.0).unwrap();
        assert!(s.asymmetry().is_infinite());
    }

    #[test]
    fn magnitude_is_what_a_flat_store_would_have_carried() {
        let one = ChiralScale::new(2, 8.0, 2.0).unwrap();
        let mut two = one;
        two.scale_up().unwrap();
        assert!(
            (one.magnitude() - two.magnitude()).abs() < 1e-4,
            "a bilateral jump preserves magnitude"
        );
    }
}
