//! Closed intervals of finite `f64` values, in SI base units.
//!
//! Limitation (design 4.1): endpoints are computed with ordinary
//! round-to-nearest `f64` operations, without directed rounding. A computed
//! endpoint can be off by a few ulps in either direction, so an `Interval`
//! is not a guaranteed enclosure. The engine compensates with the guard of
//! D-15 on the `IMPOSSIBLE` rules; tests compare with `REL_TOL` and `ABS_TOL`.

use crate::error::EngineError;

/// Closed interval `[lo, hi]` in SI base units.
///
/// Invariants: both endpoints are finite and `lo <= hi`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    lo: f64,
    hi: f64,
}

impl Interval {
    /// Builds `[lo, hi]`.
    ///
    /// Errors: `NonFiniteValue` if an endpoint is not finite,
    /// `InvertedInterval` if `lo > hi`.
    pub fn new(lo: f64, hi: f64) -> Result<Interval, EngineError> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(EngineError::NonFiniteValue { field: "interval" });
        }
        if lo > hi {
            return Err(EngineError::InvertedInterval);
        }
        Ok(Interval { lo, hi })
    }

    /// Builds the point interval `[x, x]`. Errors: `NonFiniteValue`.
    pub fn point(x: f64) -> Result<Interval, EngineError> {
        Interval::new(x, x)
    }

    /// Lower endpoint.
    pub fn lo(&self) -> f64 {
        self.lo
    }

    /// Upper endpoint.
    pub fn hi(&self) -> f64 {
        self.hi
    }

    /// `[a.lo + b.lo, a.hi + b.hi]`. Errors: `NonFiniteValue` on overflow.
    // Names and signatures follow design 4. The operations are fallible, so
    // they return `Result` and are not `std::ops` impls.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Interval) -> Result<Interval, EngineError> {
        Interval::new(self.lo + other.lo, self.hi + other.hi)
    }

    /// `[a.lo - b.hi, a.hi - b.lo]`. Errors: `NonFiniteValue` on overflow.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Interval) -> Result<Interval, EngineError> {
        Interval::new(self.lo - other.hi, self.hi - other.lo)
    }

    /// Product: min and max of the four endpoint products.
    /// Errors: `NonFiniteValue` on overflow.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Interval) -> Result<Interval, EngineError> {
        Interval::from_candidates([
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ])
    }

    /// Quotient: min and max of the four endpoint quotients.
    ///
    /// Errors: `DivisionByIntervalContainingZero` if `lo <= 0 <= hi` for
    /// `other`, `NonFiniteValue` on overflow.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, other: Interval) -> Result<Interval, EngineError> {
        if other.lo <= 0.0 && other.hi >= 0.0 {
            return Err(EngineError::DivisionByIntervalContainingZero);
        }
        Interval::from_candidates([
            self.lo / other.lo,
            self.lo / other.hi,
            self.hi / other.lo,
            self.hi / other.hi,
        ])
    }

    /// Smallest interval containing both `self` and `other`.
    pub fn hull(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    /// `[min, max]` of the candidates, rejecting any non-finite candidate.
    fn from_candidates(candidates: [f64; 4]) -> Result<Interval, EngineError> {
        if candidates.iter().any(|c| !c.is_finite()) {
            return Err(EngineError::NonFiniteValue { field: "interval" });
        }
        let lo = candidates.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = candidates.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Interval::new(lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::Interval;
    use crate::error::EngineError;

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::new(lo, hi).expect("valid interval")
    }

    #[test]
    fn new_keeps_endpoints() {
        let x = iv(-1.5, 2.0);
        assert_eq!(x.lo(), -1.5);
        assert_eq!(x.hi(), 2.0);
    }

    #[test]
    fn point_has_equal_endpoints() {
        let x = Interval::point(3.0).expect("valid point");
        assert_eq!((x.lo(), x.hi()), (3.0, 3.0));
    }

    #[test]
    fn inverted_interval_is_rejected() {
        assert_eq!(Interval::new(2.0, 1.0), Err(EngineError::InvertedInterval));
    }

    #[test]
    fn non_finite_endpoints_are_rejected() {
        let err = Err(EngineError::NonFiniteValue { field: "interval" });
        assert_eq!(Interval::new(f64::NAN, 1.0), err);
        assert_eq!(Interval::new(0.0, f64::INFINITY), err);
        assert_eq!(Interval::new(f64::NEG_INFINITY, 0.0), err);
        assert_eq!(Interval::point(f64::NAN), err);
    }

    #[test]
    fn add_on_known_values() {
        assert_eq!(iv(1.0, 2.0).add(iv(10.0, 20.0)), Ok(iv(11.0, 22.0)));
        assert_eq!(iv(-3.0, -1.0).add(iv(1.0, 1.0)), Ok(iv(-2.0, 0.0)));
    }

    #[test]
    fn sub_on_known_values() {
        assert_eq!(iv(10.0, 20.0).sub(iv(1.0, 2.0)), Ok(iv(8.0, 19.0)));
        // x - x is not [0, 0] in interval arithmetic.
        assert_eq!(iv(1.0, 2.0).sub(iv(1.0, 2.0)), Ok(iv(-1.0, 1.0)));
    }

    #[test]
    fn overflow_is_rejected() {
        let big = iv(f64::MAX, f64::MAX);
        assert_eq!(
            big.add(big),
            Err(EngineError::NonFiniteValue { field: "interval" })
        );
        assert_eq!(
            big.sub(iv(-f64::MAX, -f64::MAX)),
            Err(EngineError::NonFiniteValue { field: "interval" })
        );
    }

    #[test]
    fn mul_covers_every_sign_case() {
        // Both positive.
        assert_eq!(iv(1.0, 2.0).mul(iv(3.0, 4.0)), Ok(iv(3.0, 8.0)));
        // Mixed times positive.
        assert_eq!(iv(-1.0, 2.0).mul(iv(3.0, 4.0)), Ok(iv(-4.0, 8.0)));
        // Negative times positive.
        assert_eq!(iv(-2.0, -1.0).mul(iv(3.0, 4.0)), Ok(iv(-8.0, -3.0)));
        // Mixed times mixed.
        assert_eq!(iv(-1.0, 2.0).mul(iv(-3.0, 4.0)), Ok(iv(-6.0, 8.0)));
        // Both negative.
        assert_eq!(iv(-2.0, -1.0).mul(iv(-4.0, -3.0)), Ok(iv(3.0, 8.0)));
        // Negative times mixed.
        assert_eq!(iv(-2.0, -1.0).mul(iv(-3.0, 4.0)), Ok(iv(-8.0, 6.0)));
        // Zero point.
        assert_eq!(iv(0.0, 0.0).mul(iv(-3.0, 4.0)), Ok(iv(0.0, 0.0)));
    }

    #[test]
    fn mul_overflow_is_rejected() {
        assert_eq!(
            iv(f64::MAX, f64::MAX).mul(iv(2.0, 2.0)),
            Err(EngineError::NonFiniteValue { field: "interval" })
        );
    }

    #[test]
    fn div_on_known_values() {
        assert_eq!(iv(1.0, 2.0).div(iv(4.0, 8.0)), Ok(iv(0.125, 0.5)));
        assert_eq!(iv(-1.0, 2.0).div(iv(-2.0, -1.0)), Ok(iv(-2.0, 1.0)));
        assert_eq!(iv(-4.0, -2.0).div(iv(2.0, 4.0)), Ok(iv(-2.0, -0.5)));
    }

    #[test]
    fn div_by_interval_containing_zero_is_rejected() {
        for divisor in [
            iv(-1.0, 1.0),
            iv(0.0, 2.0),
            iv(-2.0, 0.0),
            iv(0.0, 0.0),
            iv(-0.0, 1.0),
        ] {
            assert_eq!(
                iv(1.0, 2.0).div(divisor),
                Err(EngineError::DivisionByIntervalContainingZero),
                "divisor {divisor:?}"
            );
        }
    }

    #[test]
    fn div_overflow_is_rejected() {
        assert_eq!(
            iv(f64::MAX, f64::MAX).div(iv(0.5, 0.5)),
            Err(EngineError::NonFiniteValue { field: "interval" })
        );
    }

    #[test]
    fn hull_contains_both() {
        assert_eq!(iv(1.0, 2.0).hull(iv(5.0, 6.0)), iv(1.0, 6.0));
        assert_eq!(iv(-1.0, 10.0).hull(iv(0.0, 1.0)), iv(-1.0, 10.0));
    }
}
