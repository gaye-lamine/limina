//! Input and output types of the decision engine (design 6.1, 6.2).

use std::collections::BTreeMap;

use crate::config::ConstraintsMap;
use crate::limit::{Configuration, ParameterId};
use crate::quantity::Measure;
use crate::settings::DecisionConfig;

/// A claimed value `v` with an optional uncertainty `u` (same kind as `v`,
/// or a temperature difference for an absolute temperature).
#[derive(Debug, Clone)]
pub struct Claim {
    /// The claimed value `v`.
    pub value: Measure,
    /// The uncertainty `u`; `None` means `u = 0` (R-02.2).
    pub uncertainty: Option<Measure>,
}

/// Everything `decide` needs besides the limit.
#[derive(Debug, Clone)]
pub struct DecisionInput {
    /// The claim to check.
    pub claim: Claim,
    /// Engine settings.
    pub config: DecisionConfig,
    /// Allowed options per assumption, derived upstream from the text.
    pub constraints: ConstraintsMap,
    /// Parameters supplied by the claim or the text.
    pub known_parameters: BTreeMap<ParameterId, Measure>,
}

/// Whether a missing parameter has a declared plausible domain (D-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingStatus {
    /// The limit declares no plausible domain for this parameter.
    NoDomain,
    /// The limit declares a plausible domain for this parameter.
    HasDomain,
}

/// A required parameter that was not supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingParameter {
    /// Identifier of the parameter.
    pub id: ParameterId,
    /// Whether it has a declared plausible domain.
    pub status: MissingStatus,
}

/// How certain a member of `C*` is (D-10, D-19).
///
/// `Depends` lists all missing parameters of the limit (D-13). Known
/// simplification: it may list parameters that `B(c)` does not depend on.
#[derive(Debug, Clone, PartialEq)]
pub enum Support {
    /// The claim satisfies the bound for this configuration over the whole
    /// plausible range of the missing parameters.
    Certain,
    /// Whether the claim satisfies the bound depends on these parameters.
    Depends(Vec<ParameterId>),
}

/// One member of `C*`.
#[derive(Debug, Clone, PartialEq)]
pub struct SatisfyingConfig {
    /// The configuration.
    pub config: Configuration,
    /// Its support tag.
    pub support: Support,
}

/// The verdict. Exactly one variant is returned (P-03).
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The claim exceeds the bound, by more than the guard, in every
    /// configuration of `S` (R-09).
    Impossible,
    /// The claim is possible only under the listed configurations `C*`,
    /// which is never empty (E-04).
    Conditional {
        /// `C*`, in configuration order.
        satisfying: Vec<SatisfyingConfig>,
    },
    /// The claim is within the bound even in the most restrictive
    /// configuration.
    Compatible,
    /// A missing parameter changes or prevents the verdict.
    InsufficientData {
        /// All missing parameters, tagged (D-16).
        missing: Vec<MissingParameter>,
    },
}

/// Hull `[min, max]` of `lo/B_max`, `lo/B_min`, `hi/B_max`, `hi/B_min`
/// (D-12). Unitless. Always `low <= high`.
///
/// Max-bound: a value `<= 1` is within the bound. Min-bound: a value `>= 1`
/// is within the bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    /// Smallest of the four ratios.
    pub low: f64,
    /// Largest of the four ratios.
    pub high: f64,
}

/// `B_min` and `B_max` over `S` under one enclosure, in SI base units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundRange {
    /// Minimum of the selected bound values over `S`.
    pub b_min: f64,
    /// Maximum of the selected bound values over `S`.
    pub b_max: f64,
}

/// Bounds and margin under one enclosure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosureResult {
    /// `B_min` and `B_max` under this enclosure.
    pub bounds: BoundRange,
    /// `None` when `B_min == 0` under this enclosure (D-12).
    pub margin: Option<Margin>,
}

/// Both enclosures (D-11). Equal when no parameter is missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enclosures {
    /// Permissive enclosure.
    pub permissive: EnclosureResult,
    /// Restrictive enclosure.
    pub restrictive: EnclosureResult,
}

/// The full output of `decide`.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// The verdict.
    pub verdict: Verdict,
    /// Settings actually used (`k`, guard, cap).
    pub config: DecisionConfig,
    /// `v - k*u`, SI base units.
    pub claim_lo: f64,
    /// `v + k*u`, SI base units.
    pub claim_hi: f64,
    /// `None` exactly when a missing parameter prevents computing any
    /// enclosure (D-11, R-07.4).
    pub enclosures: Option<Enclosures>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        BoundRange, Claim, Decision, DecisionInput, EnclosureResult, Enclosures, Margin,
        MissingParameter, MissingStatus, Support, Verdict,
    };
    use crate::config::ConstraintsMap;
    use crate::limit::ParameterId;
    use crate::quantity::{Measure, Quantity};
    use crate::settings::DecisionConfig;

    fn result(b_min: f64, b_max: f64) -> EnclosureResult {
        EnclosureResult {
            bounds: BoundRange { b_min, b_max },
            margin: Some(Margin {
                low: 0.5,
                high: 1.5,
            }),
        }
    }

    #[test]
    fn decision_equality_covers_every_field() {
        let a = Decision {
            verdict: Verdict::Compatible,
            config: DecisionConfig::default(),
            claim_lo: 1.0,
            claim_hi: 2.0,
            enclosures: Some(Enclosures {
                permissive: result(1.0, 2.0),
                restrictive: result(1.0, 2.0),
            }),
        };
        assert_eq!(a.clone(), a);
        let mut b = a.clone();
        b.claim_hi = 3.0;
        assert_ne!(a, b);
        let mut c = a.clone();
        c.enclosures = None;
        assert_ne!(a, c);
    }

    #[test]
    fn verdict_variants_are_distinct() {
        let missing = vec![MissingParameter {
            id: ParameterId::new("p"),
            status: MissingStatus::NoDomain,
        }];
        assert_ne!(Verdict::Impossible, Verdict::Compatible);
        assert_ne!(
            Verdict::InsufficientData {
                missing: missing.clone()
            },
            Verdict::InsufficientData { missing: vec![] }
        );
        assert_ne!(
            Support::Certain,
            Support::Depends(vec![ParameterId::new("p")])
        );
    }

    #[test]
    fn decision_input_holds_the_claim() {
        let v = Measure::Plain(Quantity::from_unit(5.0, "kW").expect("valid"));
        let input = DecisionInput {
            claim: Claim {
                value: v,
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        };
        assert_eq!(input.claim.value.si_value(), 5000.0);
        assert!(input.claim.uncertainty.is_none());
    }
}
