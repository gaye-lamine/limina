//! E-03: an empty S always returns the typed error, never a verdict.
//!
//! E-02 and E-04 are in `rules_props.rs` (written in slice 5).

mod common;

use std::collections::BTreeSet;

use common::scenario::{config, scenario, Scenario};
use limina_core::{AssumptionId, EngineError};
use proptest::prelude::*;

/// How `S` is made empty.
#[derive(Clone, Copy, Debug)]
enum Emptying {
    /// A constraint with an empty allowed set on assumption `a{index}`.
    EmptyConstraint(usize),
    /// An extra assumption whose declared domain is empty.
    EmptyDomain,
}

fn emptying() -> impl Strategy<Value = Emptying> {
    prop_oneof![
        (0usize..3).prop_map(Emptying::EmptyConstraint),
        Just(Emptying::EmptyDomain),
    ]
}

proptest! {
    #![proptest_config(config())]

    /// E-03: whatever the claim, the bounds and the guard, an empty S gives
    /// `EmptyConfigurationSet`.
    #[test]
    fn e03_empty_s_is_always_the_typed_error(s in scenario(), how in emptying()) {
        let s: Scenario = s;
        let mut limit = s.limit();
        let mut input = s.input();
        match how {
            Emptying::EmptyConstraint(i) => {
                let index = i % s.dims.len();
                input
                    .constraints
                    .allow(AssumptionId::new(format!("a{index}")), BTreeSet::new());
            }
            Emptying::EmptyDomain => {
                limit = limit.with_assumption("z_empty", &[]);
            }
        }
        let result = limina_core::decide(&limit, &input);
        prop_assert_eq!(result, Err(EngineError::EmptyConfigurationSet), "{:?} / {:?}", s, how);
    }
}
