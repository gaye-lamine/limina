//! Numerical tolerances (D-06) and the guard on `IMPOSSIBLE` (D-15).

/// Relative tolerance (D-06). Default value of `guard_rel`.
pub const REL_TOL: f64 = 1e-9;

/// Absolute tolerance in SI base units, for values at or near zero (D-06).
/// Used by `approx_eq` only; since D-24 it is no longer the default
/// `guard_abs`, which is 0.
pub const ABS_TOL: f64 = 1e-12;

/// `|a - b| <= ABS_TOL + REL_TOL * max(|a|, |b|)`.
///
/// Used by tests to compare computed values. The verdict rules do not use it.
pub fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= ABS_TOL + REL_TOL * a.abs().max(b.abs())
}

/// Guard `g(x) = max(guard_rel * |x|, guard_abs)` (D-15).
///
/// Used only by the `IMPOSSIBLE` rules and by the membership test of `C*`.
pub fn guard(x: f64, guard_rel: f64, guard_abs: f64) -> f64 {
    (guard_rel * x.abs()).max(guard_abs)
}

#[cfg(test)]
mod tests {
    use super::{approx_eq, guard, ABS_TOL, REL_TOL};

    #[test]
    fn constants_match_the_spec() {
        assert_eq!(REL_TOL, 1e-9);
        assert_eq!(ABS_TOL, 1e-12);
    }

    #[test]
    fn approx_eq_near_zero_uses_the_absolute_tolerance() {
        assert!(approx_eq(0.0, 0.0));
        assert!(approx_eq(0.0, 5e-13));
        assert!(!approx_eq(0.0, 1e-11));
    }

    #[test]
    fn approx_eq_at_large_values_uses_the_relative_tolerance() {
        assert!(approx_eq(1e12, 1e12 + 100.0));
        assert!(!approx_eq(1e12, 1e12 + 1e4));
        assert!(approx_eq(-1e12, -1e12 - 100.0));
    }

    #[test]
    fn approx_eq_is_symmetric() {
        assert_eq!(approx_eq(1.0, 1.0 + 1e-10), approx_eq(1.0 + 1e-10, 1.0));
    }

    #[test]
    fn guard_takes_the_larger_term() {
        // Relative term dominates.
        assert_eq!(guard(1e6, REL_TOL, ABS_TOL), 1e6 * REL_TOL);
        // Absolute term dominates.
        assert_eq!(guard(0.0, REL_TOL, ABS_TOL), ABS_TOL);
        assert_eq!(guard(1e-6, REL_TOL, ABS_TOL), ABS_TOL);
    }

    #[test]
    fn guard_uses_the_absolute_value_of_x() {
        assert_eq!(guard(-5.0, 0.1, 0.0), guard(5.0, 0.1, 0.0));
        assert_eq!(guard(-5.0, 0.1, 0.0), 0.1 * 5.0);
    }

    #[test]
    fn zero_guard_is_zero() {
        assert_eq!(guard(123.0, 0.0, 0.0), 0.0);
    }
}
