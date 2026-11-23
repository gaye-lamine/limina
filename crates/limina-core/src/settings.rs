//! Engine settings (R-02.3, D-15, D-17, D-18, D-20, D-21, D-24).

use crate::error::EngineError;
use crate::tolerance::REL_TOL;

/// Settings of the decision engine. The guard is global (D-20), plus an
/// optional per-limit absolute floor, `Limit::guard_abs` (D-24).
///
/// Fields are private and validated on construction, so a `DecisionConfig`
/// value is always valid:
/// - `k`: coverage factor, finite and `> 0`;
/// - `guard_rel`: finite and `0 <= guard_rel < 1` (D-18);
/// - `guard_abs`: finite and `>= 0`, in SI base units;
/// - `max_configurations`: cap on `|S|`, `>= 1` (D-17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionConfig {
    k: f64,
    guard_rel: f64,
    guard_abs: f64,
    max_configurations: usize,
}

impl DecisionConfig {
    /// Default coverage factor (R-02.3).
    pub const DEFAULT_K: f64 = 2.0;

    /// Default absolute term of the guard: 0, so the default guard is
    /// relative only (D-24).
    pub const DEFAULT_GUARD_ABS: f64 = 0.0;

    /// Default cap on `|S|` (D-17).
    pub const DEFAULT_MAX_CONFIGURATIONS: usize = 100_000;

    /// Builds a validated configuration.
    ///
    /// Errors, checked in this order:
    /// `InvalidCoverageFactor` if `k` is not finite or `<= 0`;
    /// `InvalidGuard` if `guard_rel` is not finite or not in `[0, 1)` (D-18);
    /// `InvalidGuard` if `guard_abs` is not finite or `< 0`;
    /// `InvalidMaxConfigurations` if `max_configurations == 0`.
    pub fn new(
        k: f64,
        guard_rel: f64,
        guard_abs: f64,
        max_configurations: usize,
    ) -> Result<DecisionConfig, EngineError> {
        if !k.is_finite() || k <= 0.0 {
            return Err(EngineError::InvalidCoverageFactor);
        }
        if !guard_rel.is_finite() || !(0.0..1.0).contains(&guard_rel) {
            return Err(EngineError::InvalidGuard);
        }
        if !guard_abs.is_finite() || guard_abs < 0.0 {
            return Err(EngineError::InvalidGuard);
        }
        if max_configurations == 0 {
            return Err(EngineError::InvalidMaxConfigurations);
        }
        Ok(DecisionConfig {
            k,
            guard_rel,
            guard_abs,
            max_configurations,
        })
    }

    /// Coverage factor `k`.
    pub fn k(&self) -> f64 {
        self.k
    }

    /// Relative term of the guard.
    pub fn guard_rel(&self) -> f64 {
        self.guard_rel
    }

    /// Absolute term of the guard, in SI base units.
    pub fn guard_abs(&self) -> f64 {
        self.guard_abs
    }

    /// Cap on `|S|`.
    pub fn max_configurations(&self) -> usize {
        self.max_configurations
    }
}

impl Default for DecisionConfig {
    /// `k = 2`, `guard_rel = REL_TOL`, `guard_abs = 0` (D-24),
    /// `max_configurations = 100000`.
    fn default() -> DecisionConfig {
        DecisionConfig {
            k: DecisionConfig::DEFAULT_K,
            guard_rel: REL_TOL,
            guard_abs: DecisionConfig::DEFAULT_GUARD_ABS,
            max_configurations: DecisionConfig::DEFAULT_MAX_CONFIGURATIONS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DecisionConfig;
    use crate::error::EngineError;
    use crate::tolerance::{ABS_TOL, REL_TOL};

    fn build(k: f64, rel: f64, abs: f64, max: usize) -> Result<DecisionConfig, EngineError> {
        DecisionConfig::new(k, rel, abs, max)
    }

    #[test]
    fn defaults_match_the_spec() {
        let c = DecisionConfig::default();
        assert_eq!(c.k(), 2.0);
        assert_eq!(c.guard_rel(), REL_TOL);
        assert_eq!(c.guard_abs(), 0.0);
        assert_eq!(c.max_configurations(), 100_000);
    }

    #[test]
    fn defaults_pass_validation() {
        let d = DecisionConfig::default();
        assert_eq!(
            build(d.k(), d.guard_rel(), d.guard_abs(), d.max_configurations()),
            Ok(d)
        );
    }

    #[test]
    fn invalid_coverage_factor_is_rejected() {
        for k in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                build(k, REL_TOL, ABS_TOL, 1),
                Err(EngineError::InvalidCoverageFactor),
                "k = {k}"
            );
        }
    }

    #[test]
    fn guard_rel_outside_zero_one_is_rejected() {
        for rel in [1.0, 1.5, -1e-9, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                build(2.0, rel, ABS_TOL, 1),
                Err(EngineError::InvalidGuard),
                "guard_rel = {rel}"
            );
        }
    }

    #[test]
    fn guard_rel_inside_zero_one_is_accepted() {
        for rel in [0.0, 0.999, REL_TOL] {
            assert!(build(2.0, rel, ABS_TOL, 1).is_ok(), "guard_rel = {rel}");
        }
    }

    #[test]
    fn invalid_guard_abs_is_rejected() {
        for abs in [-1e-12, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                build(2.0, REL_TOL, abs, 1),
                Err(EngineError::InvalidGuard),
                "guard_abs = {abs}"
            );
        }
        assert!(build(2.0, REL_TOL, 0.0, 1).is_ok());
    }

    #[test]
    fn zero_max_configurations_is_rejected() {
        assert_eq!(
            build(2.0, REL_TOL, ABS_TOL, 0),
            Err(EngineError::InvalidMaxConfigurations)
        );
        assert!(build(2.0, REL_TOL, ABS_TOL, 1).is_ok());
    }

    #[test]
    fn getters_return_the_validated_values() {
        let c = build(3.0, 0.5, 0.25, 7).expect("valid");
        assert_eq!(
            (c.k(), c.guard_rel(), c.guard_abs(), c.max_configurations()),
            (3.0, 0.5, 0.25, 7)
        );
    }
}
