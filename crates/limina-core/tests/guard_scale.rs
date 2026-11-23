//! D-24: the default guard is relative only, so IMPOSSIBLE is reachable at
//! every scale; a limit may declare its own absolute floor.
//!
//! `ScaleLimit` is a test limit with one point bound shared by every option.
//! It encodes no physics beyond the Landauer-sized number below.
mod common;

use std::collections::BTreeMap;

use common::scenario::{config, SEED};
use limina_core::{
    decide, Assumption, AssumptionId, AssumptionOption, Claim, Configuration, ConstraintsMap,
    DecisionConfig, DecisionInput, Dimension, Direction, EngineError, Interval, Limit, Measure,
    MeasureKind, OptionId, ParameterEnclosure, ParameterSpec, Quantity, Verdict,
};
use proptest::prelude::*;

/// Joule: m^2 kg s^-2.
const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);
/// kB * 300 K * ln 2, in J.
const LANDAUER_300K: f64 = 1.380649e-23 * 300.0 * std::f64::consts::LN_2;

struct ScaleLimit {
    direction: Direction,
    dimension: Dimension,
    bound: f64,
    floor: f64,
    assumptions: Vec<Assumption>,
}

impl ScaleLimit {
    fn new(direction: Direction, dimension: Dimension, bound: f64, options: usize) -> ScaleLimit {
        let domain = (0..options)
            .map(|i| AssumptionOption {
                id: OptionId::new(i.to_string()),
                value: None,
            })
            .collect();
        ScaleLimit {
            direction,
            dimension,
            bound,
            floor: 0.0,
            assumptions: vec![Assumption {
                id: AssumptionId::new("mode"),
                domain,
            }],
        }
    }

    fn with_floor(mut self, floor: f64) -> ScaleLimit {
        self.floor = floor;
        self
    }
}

impl Limit for ScaleLimit {
    fn id(&self) -> &str {
        "scale"
    }
    fn direction(&self) -> Direction {
        self.direction
    }
    fn bounded_kind(&self) -> MeasureKind {
        MeasureKind::Plain(self.dimension)
    }
    fn assumptions(&self) -> &[Assumption] {
        &self.assumptions
    }
    fn parameters(&self) -> &[ParameterSpec] {
        &[]
    }
    fn bound(&self, _: &Configuration, _: &ParameterEnclosure) -> Result<Interval, EngineError> {
        Interval::point(self.bound)
    }
    fn guard_abs(&self) -> f64 {
        self.floor
    }
}

fn decide_with(
    limit: &ScaleLimit,
    v: f64,
    unit: &str,
    cfg: DecisionConfig,
) -> Result<Verdict, EngineError> {
    let input = DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_unit(v, unit)?),
            uncertainty: None,
        },
        config: cfg,
        constraints: ConstraintsMap::new(),
        known_parameters: BTreeMap::new(),
    };
    Ok(decide(limit, &input)?.verdict)
}

fn verdict(limit: &ScaleLimit, v: f64, unit: &str) -> Verdict {
    decide_with(limit, v, unit, DecisionConfig::default()).expect("valid input")
}

#[test]
fn landauer_sized_bound_reaches_impossible_and_compatible() {
    let limit = ScaleLimit::new(Direction::Min, ENERGY, LANDAUER_300K, 1);
    assert!((LANDAUER_300K - 2.871e-21).abs() < 1e-24);
    assert_eq!(
        verdict(&limit, LANDAUER_300K / 100.0, "J"),
        Verdict::Impossible
    );
    assert_eq!(
        verdict(&limit, LANDAUER_300K * 100.0, "J"),
        Verdict::Compatible
    );
    assert_ne!(verdict(&limit, LANDAUER_300K, "J"), Verdict::Impossible);
}

#[test]
fn a_zero_bound_with_a_declared_floor_keeps_the_previous_behaviour() {
    // The two regression cases of slice 5, now with the floor declared by
    // the limit instead of the settings: v within 1e-12 of the bound 0 is
    // never IMPOSSIBLE, and one ulp past the floor is.
    let limit = ScaleLimit::new(Direction::Max, Dimension::DIMENSIONLESS, 0.0, 1).with_floor(1e-12);
    for v in [1e-12, 5.498937935469428e-13] {
        assert!(
            matches!(
                verdict(&limit, v, "dimensionless"),
                Verdict::Conditional { .. }
            ),
            "v = {v:e}"
        );
    }
    assert_eq!(
        verdict(&limit, 1e-12_f64.next_up(), "dimensionless"),
        Verdict::Impossible
    );
    // Without the floor, any v > 0 is above the bound 0.
    let bare = ScaleLimit::new(Direction::Max, Dimension::DIMENSIONLESS, 0.0, 1);
    assert_eq!(verdict(&bare, 1e-12, "dimensionless"), Verdict::Impossible);
}

#[test]
fn the_larger_of_the_settings_and_the_limit_floor_applies() {
    let limit = ScaleLimit::new(Direction::Max, Dimension::DIMENSIONLESS, 0.0, 1).with_floor(1e-12);
    let cfg = DecisionConfig::new(2.0, 1e-9, 1e-6, 100_000).expect("valid settings");
    assert_ne!(
        decide_with(&limit, 1e-7, "dimensionless", cfg).expect("valid"),
        Verdict::Impossible
    );
    let cfg = DecisionConfig::new(2.0, 1e-9, 1e-15, 100_000).expect("valid settings");
    assert_eq!(
        decide_with(&limit, 1e-11, "dimensionless", cfg).expect("valid"),
        Verdict::Impossible
    );
}

#[test]
fn an_invalid_declared_floor_is_rejected() {
    for floor in [-1e-12, f64::NAN, f64::INFINITY] {
        let limit =
            ScaleLimit::new(Direction::Max, Dimension::DIMENSIONLESS, 1.0, 1).with_floor(floor);
        assert_eq!(
            decide_with(&limit, 2.0, "dimensionless", DecisionConfig::default()),
            Err(EngineError::InvalidGuard),
            "floor = {floor}"
        );
    }
}

/// A bound of any scale in [1e-30, 1e30], shared by 1 to 3 options.
fn bound_and_options() -> impl Strategy<Value = (f64, usize)> {
    ((-30.0..=30.0f64).prop_map(|e| 10f64.powf(e)), 1usize..=3)
}

proptest! {
    #![proptest_config(config())]

    /// Max: a claim at or below b is never IMPOSSIBLE; a claim above
    /// b * (1 + 1e-6) is IMPOSSIBLE.
    #[test]
    fn max_bound_decides_at_every_scale(
        (b, options) in bound_and_options(),
        below in 0.0..=1.0f64,
        above in 0.0..=10.0f64,
    ) {
        let limit = ScaleLimit::new(Direction::Max, Dimension::DIMENSIONLESS, b, options);
        let low = b * below;
        prop_assert_ne!(
            verdict(&limit, low, "dimensionless"), Verdict::Impossible,
            "seed {:#x}: b = {:e}, options = {}, claim = {:e}", SEED, b, options, low
        );
        let high = b * (1.0 + 1e-6) * (1.0 + above);
        let high = if high > b * (1.0 + 1e-6) { high } else { high.next_up() };
        prop_assert_eq!(
            verdict(&limit, high, "dimensionless"), Verdict::Impossible,
            "seed {:#x}: b = {:e}, options = {}, claim = {:e}", SEED, b, options, high
        );
    }

    /// Min, the mirror: a claim at or above b is never IMPOSSIBLE; a claim
    /// below b * (1 - 1e-6) is IMPOSSIBLE.
    #[test]
    fn min_bound_decides_at_every_scale(
        (b, options) in bound_and_options(),
        above in 1.0..=10.0f64,
        below in 0.0..=1.0f64,
    ) {
        let limit = ScaleLimit::new(Direction::Min, ENERGY, b, options);
        let high = b * above;
        prop_assert_ne!(
            verdict(&limit, high, "J"), Verdict::Impossible,
            "seed {:#x}: b = {:e}, options = {}, claim = {:e}", SEED, b, options, high
        );
        let low = (b * (1.0 - 1e-6) * below).next_down().max(0.0);
        prop_assert_eq!(
            verdict(&limit, low, "J"), Verdict::Impossible,
            "seed {:#x}: b = {:e}, options = {}, claim = {:e}", SEED, b, options, low
        );
    }
}
