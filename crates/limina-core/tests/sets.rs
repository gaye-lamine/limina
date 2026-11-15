//! P-01 (widening S never lowers B_max nor raises B_min) and P-10 (the same
//! input gives an identical output).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::scenario::{config, scenario};
use common::TestLimit;
use limina_core::{
    bound_range, AssumptionId, ConfigurationSet, ConstraintsMap, Direction, EnclosureKind,
    OptionId, ParameterEnclosure,
};
use proptest::prelude::*;

/// A limit with interval bounds, and two constraint maps where `wide`
/// allows a superset of the options of `narrow` for every assumption.
#[derive(Clone, Debug)]
struct WideningCase {
    direction: Direction,
    /// Options per assumption.
    dims: Vec<usize>,
    /// `[lo, hi]` of `B(c)` per configuration key.
    bounds: BTreeMap<Vec<String>, (f64, f64)>,
    /// Allowed option indices per assumption.
    narrow: Vec<BTreeSet<usize>>,
    wide: Vec<BTreeSet<usize>>,
}

impl WideningCase {
    fn limit(&self) -> TestLimit {
        let mut l = TestLimit::new(self.direction);
        for (i, n) in self.dims.iter().enumerate() {
            let options: Vec<String> = (0..*n).map(|o| o.to_string()).collect();
            let refs: Vec<&str> = options.iter().map(String::as_str).collect();
            l = l.with_assumption(&format!("a{i}"), &refs);
        }
        for (key, (lo, hi)) in &self.bounds {
            let refs: Vec<&str> = key.iter().map(String::as_str).collect();
            l = l.with_bound(&refs, *lo, *hi);
        }
        l
    }
}

fn constraints(allowed: &[BTreeSet<usize>]) -> ConstraintsMap {
    let mut c = ConstraintsMap::new();
    for (i, set) in allowed.iter().enumerate() {
        let options: BTreeSet<OptionId> =
            set.iter().map(|o| OptionId::new(o.to_string())).collect();
        c.allow(AssumptionId::new(format!("a{i}")), options);
    }
    c
}

fn keys(dims: &[usize]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = vec![Vec::new()];
    for n in dims {
        out = out
            .into_iter()
            .flat_map(|prefix| {
                (0..*n).map(move |o| {
                    let mut next = prefix.clone();
                    next.push(o.to_string());
                    next
                })
            })
            .collect();
    }
    out
}

fn mask_to_set(mask: u8, n: usize) -> BTreeSet<usize> {
    (0..n).filter(|i| mask & (1 << i) != 0).collect()
}

fn widening_case() -> impl Strategy<Value = WideningCase> {
    (
        any::<bool>(),
        proptest::collection::vec(1usize..=3, 1..=3),
        proptest::collection::vec((0u32..=1000, 0u32..=100), 27),
        proptest::collection::vec(0u8..8, 3),
        proptest::collection::vec(0u8..8, 3),
    )
        .prop_map(|(is_max, dims, raw, narrow_masks, extra_masks)| {
            let bounds = keys(&dims)
                .into_iter()
                .zip(raw.iter().map(|(b, w)| (f64::from(*b), f64::from(b + w))))
                .collect();
            let mut narrow = Vec::new();
            let mut wide = Vec::new();
            for (i, n) in dims.iter().enumerate() {
                let full = (1u8 << n) - 1;
                let mut m = narrow_masks[i] & full;
                if m == 0 {
                    m = 1; // keep S non-empty
                }
                narrow.push(mask_to_set(m, *n));
                wide.push(mask_to_set(m | (extra_masks[i] & full), *n));
            }
            WideningCase {
                direction: if is_max {
                    Direction::Max
                } else {
                    Direction::Min
                },
                dims,
                bounds,
                narrow,
                wide,
            }
        })
}

proptest! {
    #![proptest_config(config())]

    /// P-01: widening S never lowers B_max nor raises B_min, under both
    /// enclosures.
    #[test]
    fn p01_widening_s_never_lowers_b_max_nor_raises_b_min(case in widening_case()) {
        let limit = case.limit();
        let narrow = ConfigurationSet::build(&limit, &constraints(&case.narrow), 100_000)
            .expect("non-empty narrow set");
        let wide = ConfigurationSet::build(&limit, &constraints(&case.wide), 100_000)
            .expect("non-empty wide set");
        prop_assert!(wide.len() >= narrow.len());
        for kind in [EnclosureKind::Permissive, EnclosureKind::Restrictive] {
            let params = ParameterEnclosure::default();
            let n = bound_range(&limit, &narrow, &params, kind).expect("valid bounds");
            let w = bound_range(&limit, &wide, &params, kind).expect("valid bounds");
            prop_assert!(w.b_max >= n.b_max, "{kind:?}: wide {w:?} narrow {n:?}");
            prop_assert!(w.b_min <= n.b_min, "{kind:?}: wide {w:?} narrow {n:?}");
        }
    }

    /// P-10: the same input gives an identical output, including when the
    /// constraints are inserted in another order.
    #[test]
    fn p10_same_input_gives_identical_output(
        s in scenario(),
        masks in proptest::collection::vec(0u8..8, 3),
    ) {
        let limit = s.limit();
        let allowed: Vec<BTreeSet<usize>> = s
            .dims
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let m = masks[i] & ((1u8 << n) - 1);
                mask_to_set(if m == 0 { 1 } else { m }, *n)
            })
            .collect();

        let mut forward = s.input();
        forward.constraints = constraints(&allowed);

        let mut reverse = s.input();
        let mut c = ConstraintsMap::new();
        for (i, set) in allowed.iter().enumerate().rev() {
            let options: BTreeSet<OptionId> =
                set.iter().map(|o| OptionId::new(o.to_string())).collect();
            c.allow(AssumptionId::new(format!("a{i}")), options);
        }
        reverse.constraints = c;

        let first = limina_core::decide(&limit, &forward);
        let second = limina_core::decide(&limit, &forward);
        let reordered = limina_core::decide(&limit, &reverse);
        prop_assert_eq!(&first, &second);
        prop_assert_eq!(&first, &reordered);
    }
}
