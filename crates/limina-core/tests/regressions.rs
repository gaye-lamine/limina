//! Regression cases and the fixed-seed check (design 9.1).
//!
//! Every shrunk counterexample found by a property is copied here as a named
//! test with its values written out, so it keeps running even if the
//! proptest regression files are lost.

mod common;

use std::collections::BTreeMap;

use common::scenario::{config, scenario, Expected, GuardMode, Scenario};
use limina_core::{Direction, Verdict};
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;

fn single_bound_scenario(b: f64, v: f64, guard: GuardMode) -> Scenario {
    let mut bounds = BTreeMap::new();
    bounds.insert(vec!["0".to_string()], b);
    Scenario {
        direction: Direction::Max,
        dims: vec![1],
        bounds,
        v,
        u: 0.0,
        k: 1.0,
        guard,
    }
}

/// Mutation experiment of slice 5 (docs/journal.md): with the guard removed
/// from `rules_max`, P-03 (a) shrank to this case. The correct engine must
/// answer CONDITIONAL: `v` is within the absolute guard of the bound 0.
/// Since D-24 the default guard has no absolute term, so the case keeps the
/// explicit floor `abs = 1e-12` it was found with.
#[test]
fn regression_mutation_p03a_value_within_absolute_guard_of_zero_bound() {
    let s = single_bound_scenario(0.0, 1e-12, GuardMode::AbsFloor);
    let d = s.decide().expect("valid scenario");
    assert_eq!(
        common::scenario::observed(&d.verdict),
        Expected::Conditional(vec![vec!["0".to_string()]]),
        "{s:?}"
    );
}

/// Same mutation experiment, case shrunk by P-02.
#[test]
fn regression_mutation_p02_value_inside_absolute_guard_of_zero_bound() {
    let s = single_bound_scenario(0.0, 5.498937935469428e-13, GuardMode::AbsFloor);
    let d = s.decide().expect("valid scenario");
    assert_ne!(d.verdict, Verdict::Impossible, "{s:?}");
}

/// The fixed seed makes generation reproducible: two runners built from the
/// same configuration produce the same scenarios.
#[test]
fn fixed_seed_generates_the_same_scenarios() {
    let generate = || {
        let mut runner = TestRunner::new(config());
        (0..32)
            .map(|_| {
                let tree = scenario()
                    .new_tree(&mut runner)
                    .expect("strategy always produces a value");
                format!("{:?}", tree.current())
            })
            .collect::<Vec<String>>()
    };
    assert_eq!(generate(), generate());
}
