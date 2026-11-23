//! Differential properties: the DSL limits of `data/limits` and the Rust
//! limits of `limina-core` give the same bound, the same typed error, and
//! the same verdict with the same C* (spec limit-dsl, AC 1).
//!
//! Fixed seed; 256 cases locally, 1024 when `CI` is set. A failure prints
//! the shrunk case, whose `Debug` is written to be read.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use limina_core::{
    decide, AbsoluteTemperature, AssumptionId, Betz, Carnot, Claim, ConfigurationSet,
    ConstraintsMap, Cop, DecisionConfig, DecisionInput, Dimension, EngineError, Interval, Limit,
    Measure, OptionId, ParameterEnclosure, ParameterId, Quantity,
};
use limina_dsl::{default_dir, Registry};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

const SEED: u64 = 0x4C49_4D49_4E41; // "LIMINA", as in limina-core

fn config() -> Config {
    Config {
        cases: if std::env::var_os("CI").is_some() {
            1024
        } else {
            256
        },
        rng_seed: RngSeed::Fixed(SEED),
        ..Config::default()
    }
}

fn registry() -> Registry {
    Registry::load_dir(&default_dir()).expect("data/limits loads")
}

/// One generated input. `Debug` prints it on one readable line.
#[derive(Clone)]
struct Case {
    claim: f64,
    uncertainty: Option<f64>,
    /// Options allowed for the limit's assumption; None leaves it unconstrained.
    allowed: Option<Vec<&'static str>>,
    th: Option<f64>,
    tc: Option<f64>,
}

impl fmt::Debug for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = |v: Option<f64>| v.map_or_else(|| "missing".to_string(), |k| format!("{k:?} K"));
        let allowed = self.allowed.as_ref().map_or_else(
            || "unconstrained".to_string(),
            |a| format!("{{{}}}", a.join(", ")),
        );
        write!(
            f,
            "claim {:?} ± {:?} | options {allowed} | Th {} | Tc {}",
            self.claim,
            self.uncertainty.unwrap_or(0.0),
            t(self.th),
            t(self.tc)
        )
    }
}

/// Temperatures: valid pairs, swapped, equal, zero, and missing values.
fn temperature() -> impl Strategy<Value = Option<f64>> {
    prop_oneof![
        4 => (1.0f64..2000.0).prop_map(Some),
        1 => Just(Some(0.0)),
        1 => Just(Some(300.0)),
        1 => Just(None),
    ]
}

fn options_subset(
    options: &'static [&'static str],
) -> impl Strategy<Value = Option<Vec<&'static str>>> {
    prop_oneof![
        2 => Just(None),
        3 => proptest::sample::subsequence(options.to_vec(), 0..=options.len()).prop_map(Some),
    ]
}

fn case(
    options: &'static [&'static str],
    claim: impl Strategy<Value = f64>,
) -> impl Strategy<Value = Case> {
    (
        claim,
        prop_oneof![2 => Just(None), 1 => (0.0f64..0.05).prop_map(Some)],
        options_subset(options),
        temperature(),
        temperature(),
    )
        .prop_map(|(claim, uncertainty, allowed, th, tc)| Case {
            claim,
            uncertainty,
            allowed,
            th,
            tc,
        })
}

fn constraints(assumption: &str, allowed: &Option<Vec<&'static str>>) -> ConstraintsMap {
    let mut c = ConstraintsMap::new();
    if let Some(options) = allowed {
        c.allow(
            AssumptionId::new(assumption),
            options
                .iter()
                .map(|o| OptionId::new(*o))
                .collect::<BTreeSet<_>>(),
        );
    }
    c
}

fn input(case: &Case, assumption: &str) -> DecisionInput {
    let plain =
        |v: f64| Measure::Plain(Quantity::from_si(v, Dimension::DIMENSIONLESS).expect("finite"));
    let mut known = BTreeMap::new();
    for (name, v) in [("Th", case.th), ("Tc", case.tc)] {
        if let Some(k) = v {
            known.insert(
                ParameterId::new(name),
                Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(k).expect("k >= 0")),
            );
        }
    }
    DecisionInput {
        claim: Claim {
            value: plain(case.claim),
            uncertainty: case.uncertainty.map(plain),
        },
        config: DecisionConfig::default(),
        constraints: constraints(assumption, &case.allowed),
        known_parameters: known,
    }
}

fn enclosure(case: &Case) -> ParameterEnclosure {
    let mut m = BTreeMap::new();
    for (name, v) in [("Th", case.th), ("Tc", case.tc)] {
        if let Some(k) = v {
            m.insert(ParameterId::new(name), Interval::point(k).expect("finite"));
        }
    }
    ParameterEnclosure::new(m)
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
}

/// Same bound (relative 1e-12) or the same typed error, for every
/// configuration of the full set.
fn same_bounds(rust: &dyn Limit, dsl: &dyn Limit, case: &Case) -> Result<(), TestCaseError> {
    let set = ConfigurationSet::build(rust, &ConstraintsMap::new(), 64).expect("configurations");
    let params = enclosure(case);
    for config in set.iter() {
        let (a, b): (Result<Interval, EngineError>, _) =
            (rust.bound(config, &params), dsl.bound(config, &params));
        match (&a, &b) {
            (Ok(x), Ok(y)) => prop_assert!(
                close(x.lo(), y.lo()) && close(x.hi(), y.hi()),
                "{:?} under {:?}: rust {:?} dsl {:?}",
                case,
                config,
                x,
                y
            ),
            (Err(x), Err(y)) => prop_assert_eq!(x, y, "{:?} under {:?}", case, config),
            _ => prop_assert!(
                false,
                "{:?} under {:?}: rust {:?} dsl {:?}",
                case,
                config,
                a,
                b
            ),
        }
    }
    Ok(())
}

/// Same verdict (including C*) or the same typed error from `decide`.
fn same_decision(
    rust: &dyn Limit,
    dsl: &dyn Limit,
    case: &Case,
    assumption: &str,
) -> Result<(), TestCaseError> {
    let i = input(case, assumption);
    let a = decide(rust, &i).map(|d| d.verdict);
    let b = decide(dsl, &i).map(|d| d.verdict);
    prop_assert_eq!(a, b, "{:?}", case);
    Ok(())
}

const BETZ: &[&str] = &[
    "unshrouded_rotor",
    "shrouded_total_capture_area",
    "shrouded_rotor_area_only",
];
const CARNOT: &[&str] = &["ideal_reversible_two_reservoirs"];
const COP: &[&str] = &["heating", "cooling"];

/// Claims spread over the range of each limit, with the exact bounds and
/// their float neighbours favoured.
fn betz_claim() -> impl Strategy<Value = f64> {
    let b = 16.0f64 / 27.0;
    prop_oneof![3 => 0.0f64..1.5, 1 => Just(b), 1 => Just(b.next_up()), 1 => Just(b.next_down())]
}

fn carnot_claim() -> impl Strategy<Value = f64> {
    prop_oneof![3 => 0.0f64..1.2, 1 => Just(0.5), 1 => Just(1.0)]
}

fn cop_claim() -> impl Strategy<Value = f64> {
    prop_oneof![3 => 0.0f64..40.0, 1 => Just(14.0), 1 => Just(15.0), 1 => Just(14.5)]
}

/// A COP case whose claim sits exactly on one of its two bounds, or a float
/// step away, for valid temperatures.
fn cop_edge_case() -> impl Strategy<Value = Case> {
    (
        1.0f64..1000.0,
        1.0f64..500.0,
        0usize..2,
        -1i8..=1,
        options_subset(COP),
    )
        .prop_map(|(tc, gap, which, step, allowed)| {
            let th = tc + gap;
            let bound = if which == 0 {
                th / (th - tc)
            } else {
                tc / (th - tc)
            };
            let claim = match step {
                -1 => bound.next_down(),
                1 => bound.next_up(),
                _ => bound,
            };
            Case {
                claim,
                uncertainty: None,
                allowed,
                th: Some(th),
                tc: Some(tc),
            }
        })
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn betz_same_bound_error_verdict_and_c_star(case in case(BETZ, betz_claim())) {
        let r = registry();
        let dsl = r.get("betz").expect("betz");
        same_bounds(&Betz::new(), dsl, &case)?;
        same_decision(&Betz::new(), dsl, &case, "reference_area")?;
    }

    #[test]
    fn carnot_same_bound_error_verdict_and_c_star(case in case(CARNOT, carnot_claim())) {
        let r = registry();
        let dsl = r.get("carnot").expect("carnot");
        same_bounds(&Carnot::new(), dsl, &case)?;
        same_decision(&Carnot::new(), dsl, &case, "cycle_model")?;
    }

    #[test]
    fn cop_same_bound_error_verdict_and_c_star(case in case(COP, cop_claim())) {
        let r = registry();
        let dsl = r.get("cop").expect("cop");
        same_bounds(&Cop::new(), dsl, &case)?;
        same_decision(&Cop::new(), dsl, &case, "mode")?;
    }

    #[test]
    fn cop_same_verdict_on_the_exact_bounds(case in cop_edge_case()) {
        let r = registry();
        let dsl = r.get("cop").expect("cop");
        same_bounds(&Cop::new(), dsl, &case)?;
        same_decision(&Cop::new(), dsl, &case, "mode")?;
    }
}

#[test]
fn a_missing_temperature_is_the_same_unknown_parameter_error() {
    let r = registry();
    for (rust, id) in [
        (&Carnot::new() as &dyn Limit, "carnot"),
        (&Cop::new() as &dyn Limit, "cop"),
    ] {
        let dsl = r.get(id).expect("limit");
        let set = ConfigurationSet::build(rust, &ConstraintsMap::new(), 8).expect("set");
        for c in set.iter() {
            let empty = ParameterEnclosure::default();
            assert_eq!(rust.bound(c, &empty), dsl.bound(c, &empty));
            assert_eq!(
                dsl.bound(c, &empty),
                Err(EngineError::UnknownParameter("Th".into()))
            );
        }
    }
}

#[test]
fn the_declarations_match_the_rust_limits() {
    let r = registry();
    for (rust, id) in [
        (&Betz::new() as &dyn Limit, "betz"),
        (&Carnot::new() as &dyn Limit, "carnot"),
        (&Cop::new() as &dyn Limit, "cop"),
    ] {
        let dsl = r.get(id).expect("limit");
        assert_eq!(rust.id(), dsl.id());
        assert_eq!(rust.direction(), dsl.direction());
        assert_eq!(rust.bounded_kind(), dsl.bounded_kind());
        let ids = |l: &dyn Limit| -> Vec<(String, Vec<String>)> {
            l.assumptions()
                .iter()
                .map(|a| {
                    (
                        a.id.as_str().to_string(),
                        a.domain.iter().map(|o| o.id.as_str().to_string()).collect(),
                    )
                })
                .collect()
        };
        assert_eq!(ids(rust), ids(dsl), "{id}");
        let params = |l: &dyn Limit| -> Vec<(String, String, bool, bool)> {
            l.parameters()
                .iter()
                .map(|p| {
                    (
                        p.id.as_str().to_string(),
                        format!("{:?}", p.kind),
                        p.required,
                        p.plausible.is_some(),
                    )
                })
                .collect()
        };
        assert_eq!(params(rust), params(dsl), "{id}");
    }
}
