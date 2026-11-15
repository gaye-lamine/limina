//! Constraints map and the configuration set `S` (R-08, D-07, D-08, D-17).

use std::collections::{BTreeMap, BTreeSet};

use crate::error::EngineError;
use crate::limit::{AssumptionId, Configuration, Limit, OptionId};

/// Assumption id to allowed option ids, derived upstream from the text.
/// An assumption without an entry keeps its full declared domain.
#[derive(Debug, Clone, Default)]
pub struct ConstraintsMap(BTreeMap<AssumptionId, BTreeSet<OptionId>>);

impl ConstraintsMap {
    /// Empty map: every assumption keeps its full domain.
    pub fn new() -> ConstraintsMap {
        ConstraintsMap::default()
    }

    /// Restricts `assumption` to `options`. Replaces any earlier entry for the
    /// same assumption.
    pub fn allow(&mut self, assumption: AssumptionId, options: BTreeSet<OptionId>) {
        self.0.insert(assumption, options);
    }
}

/// The configuration set `S`: the product of the declared domains narrowed by
/// the constraints map.
///
/// Invariants: non-empty, at most `max_configurations` long, in lexicographic
/// order of configurations.
#[derive(Debug, Clone)]
pub struct ConfigurationSet(Vec<Configuration>);

impl ConfigurationSet {
    /// Builds `S`.
    ///
    /// Checks, in this order (design 5.1):
    /// 1. every constrained assumption is declared, else `UnknownAssumption`;
    /// 2. every constrained option is in its declared domain, else
    ///    `ConstraintOutsideDomain`;
    /// 3. no narrowed domain is empty, else `EmptyConfigurationSet`;
    /// 4. `|S|`, computed with checked multiplication before any
    ///    configuration is created, is at most `max_configurations`, else
    ///    `ConfigurationSetTooLarge` (an overflow counts as too large).
    ///
    /// Option ids repeated in a declared domain count once.
    pub fn build(
        limit: &dyn Limit,
        constraints: &ConstraintsMap,
        max_configurations: usize,
    ) -> Result<ConfigurationSet, EngineError> {
        let declared = declared_domains(limit);
        check_constraints(&declared, constraints)?;
        let narrowed = narrow(declared, constraints);
        if narrowed.values().any(BTreeSet::is_empty) {
            return Err(EngineError::EmptyConfigurationSet);
        }
        check_size(&narrowed, max_configurations)?;
        Ok(ConfigurationSet(enumerate(&narrowed)))
    }

    /// Configurations in lexicographic order.
    pub fn iter(&self) -> impl Iterator<Item = &Configuration> {
        self.0.iter()
    }

    /// `|S|`.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false` for a built set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

type Domains = BTreeMap<AssumptionId, BTreeSet<OptionId>>;

fn declared_domains(limit: &dyn Limit) -> Domains {
    limit
        .assumptions()
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                a.domain.iter().map(|o| o.id.clone()).collect(),
            )
        })
        .collect()
}

fn check_constraints(declared: &Domains, constraints: &ConstraintsMap) -> Result<(), EngineError> {
    for (assumption, allowed) in &constraints.0 {
        let domain = declared
            .get(assumption)
            .ok_or_else(|| EngineError::UnknownAssumption(assumption.as_str().to_string()))?;
        if let Some(option) = allowed.iter().find(|o| !domain.contains(*o)) {
            return Err(EngineError::ConstraintOutsideDomain {
                assumption: assumption.as_str().to_string(),
                option: option.as_str().to_string(),
            });
        }
    }
    Ok(())
}

fn narrow(declared: Domains, constraints: &ConstraintsMap) -> Domains {
    declared
        .into_iter()
        .map(|(id, domain)| match constraints.0.get(&id) {
            Some(allowed) => (id, &domain & allowed),
            None => (id, domain),
        })
        .collect()
}

fn check_size(domains: &Domains, max_configurations: usize) -> Result<(), EngineError> {
    let too_large = EngineError::ConfigurationSetTooLarge {
        max: max_configurations,
    };
    let mut size: usize = 1;
    for domain in domains.values() {
        size = size.checked_mul(domain.len()).ok_or(too_large.clone())?;
    }
    if size > max_configurations {
        return Err(too_large);
    }
    Ok(())
}

/// Cartesian product in lexicographic order. The first assumption (in id
/// order) varies slowest, so the output is sorted.
fn enumerate(domains: &Domains) -> Vec<Configuration> {
    let mut partial: Vec<BTreeMap<AssumptionId, OptionId>> = vec![BTreeMap::new()];
    for (id, options) in domains {
        partial = partial
            .into_iter()
            .flat_map(|prefix| {
                options.iter().map(move |o| {
                    let mut next = prefix.clone();
                    next.insert(id.clone(), o.clone());
                    next
                })
            })
            .collect();
    }
    partial.into_iter().map(Configuration::from_map).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ConfigurationSet, ConstraintsMap};
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
        ParameterEnclosure, ParameterSpec,
    };
    use crate::quantity::MeasureKind;

    /// Fake limit for these tests only: declares assumptions, no physics.
    struct FakeLimit(Vec<Assumption>);

    impl FakeLimit {
        fn new(assumptions: &[(&str, &[&str])]) -> FakeLimit {
            FakeLimit(
                assumptions
                    .iter()
                    .map(|(id, options)| Assumption {
                        id: AssumptionId::new(*id),
                        domain: options
                            .iter()
                            .map(|o| AssumptionOption {
                                id: OptionId::new(*o),
                                value: None,
                            })
                            .collect(),
                    })
                    .collect(),
            )
        }
    }

    impl Limit for FakeLimit {
        fn id(&self) -> &str {
            "fake"
        }
        fn direction(&self) -> Direction {
            Direction::Max
        }
        fn bounded_kind(&self) -> MeasureKind {
            MeasureKind::Plain(Dimension::DIMENSIONLESS)
        }
        fn assumptions(&self) -> &[Assumption] {
            &self.0
        }
        fn parameters(&self) -> &[ParameterSpec] {
            &[]
        }
        fn bound(
            &self,
            _: &Configuration,
            _: &ParameterEnclosure,
        ) -> Result<Interval, EngineError> {
            Interval::point(1.0)
        }
    }

    fn allow(map: &mut ConstraintsMap, assumption: &str, options: &[&str]) {
        let set: BTreeSet<OptionId> = options.iter().map(|o| OptionId::new(*o)).collect();
        map.allow(AssumptionId::new(assumption), set);
    }

    /// Each configuration as its option ids, in assumption id order.
    fn options_of(set: &ConfigurationSet) -> Vec<Vec<String>> {
        set.iter()
            .map(|c| c.iter().map(|(_, o)| o.as_str().to_string()).collect())
            .collect()
    }

    #[test]
    fn unconstrained_assumptions_keep_their_full_domain() {
        let limit = FakeLimit::new(&[("a", &["x", "y"]), ("b", &["p", "q", "r"])]);
        let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100).expect("valid");
        assert_eq!(set.len(), 6);
        assert!(!set.is_empty());
    }

    #[test]
    fn constraints_narrow_the_domain() {
        let limit = FakeLimit::new(&[("a", &["x", "y"]), ("b", &["p", "q", "r"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "b", &["q"]);
        let set = ConfigurationSet::build(&limit, &c, 100).expect("valid");
        assert_eq!(options_of(&set), vec![vec!["x", "q"], vec!["y", "q"]]);
    }

    #[test]
    fn configurations_are_in_lexicographic_order() {
        let limit = FakeLimit::new(&[("b", &["2", "1"]), ("a", &["y", "x"])]);
        let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100).expect("valid");
        let configs: Vec<&Configuration> = set.iter().collect();
        assert!(configs.windows(2).all(|w| w[0] < w[1]));
        // Assumption "a" sorts first, so it varies slowest.
        assert_eq!(
            options_of(&set),
            vec![
                vec!["x", "1"],
                vec!["x", "2"],
                vec!["y", "1"],
                vec!["y", "2"]
            ]
        );
    }

    #[test]
    fn unknown_assumption_is_rejected() {
        let limit = FakeLimit::new(&[("a", &["x"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "zzz", &["x"]);
        assert_eq!(
            ConfigurationSet::build(&limit, &c, 100).map(|s| s.len()),
            Err(EngineError::UnknownAssumption("zzz".to_string()))
        );
    }

    #[test]
    fn constraint_outside_domain_is_rejected() {
        let limit = FakeLimit::new(&[("a", &["x", "y"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "a", &["x", "nope"]);
        assert_eq!(
            ConfigurationSet::build(&limit, &c, 100).map(|s| s.len()),
            Err(EngineError::ConstraintOutsideDomain {
                assumption: "a".to_string(),
                option: "nope".to_string(),
            })
        );
    }

    #[test]
    fn outside_domain_is_checked_before_emptiness() {
        // The constraint names only an undeclared option, so the narrowed set
        // would also be empty; the domain error wins.
        let limit = FakeLimit::new(&[("a", &["x"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "a", &["nope"]);
        assert!(matches!(
            ConfigurationSet::build(&limit, &c, 100),
            Err(EngineError::ConstraintOutsideDomain { .. })
        ));
    }

    #[test]
    fn empty_allowed_set_gives_empty_configuration_set() {
        let limit = FakeLimit::new(&[("a", &["x", "y"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "a", &[]);
        assert_eq!(
            ConfigurationSet::build(&limit, &c, 100).map(|s| s.len()),
            Err(EngineError::EmptyConfigurationSet)
        );
    }

    #[test]
    fn empty_declared_domain_gives_empty_configuration_set() {
        let limit = FakeLimit::new(&[("a", &["x"]), ("b", &[])]);
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100).map(|s| s.len()),
            Err(EngineError::EmptyConfigurationSet)
        );
    }

    #[test]
    fn above_the_cap_is_rejected() {
        let limit = FakeLimit::new(&[("a", &["1", "2", "3"]), ("b", &["1", "2"])]);
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), 5).map(|s| s.len()),
            Err(EngineError::ConfigurationSetTooLarge { max: 5 })
        );
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), 6).map(|s| s.len()),
            Ok(6)
        );
    }

    #[test]
    fn huge_product_is_rejected_without_enumeration() {
        // 5 assumptions x 100 options = 1e10 configurations (no usize
        // overflow): enumerating them would exhaust memory, so the size check
        // must reject the set before any enumeration.
        let options: Vec<String> = (0..100).map(|i| i.to_string()).collect();
        let option_refs: Vec<&str> = options.iter().map(String::as_str).collect();
        let names: Vec<String> = (0..5).map(|i| format!("a{i}")).collect();
        let spec: Vec<(&str, &[&str])> = names
            .iter()
            .map(|n| (n.as_str(), option_refs.as_slice()))
            .collect();
        let limit = FakeLimit::new(&spec);
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100_000).map(|s| s.len()),
            Err(EngineError::ConfigurationSetTooLarge { max: 100_000 })
        );
    }

    #[test]
    fn usize_overflow_is_too_large_and_does_not_panic() {
        // 20 assumptions x 100 options = 1e40 > usize::MAX on 64-bit.
        let options: Vec<String> = (0..100).map(|i| i.to_string()).collect();
        let option_refs: Vec<&str> = options.iter().map(String::as_str).collect();
        let names: Vec<String> = (0..20).map(|i| format!("a{i:02}")).collect();
        let spec: Vec<(&str, &[&str])> = names
            .iter()
            .map(|n| (n.as_str(), option_refs.as_slice()))
            .collect();
        let limit = FakeLimit::new(&spec);
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), usize::MAX).map(|s| s.len()),
            Err(EngineError::ConfigurationSetTooLarge { max: usize::MAX })
        );
    }

    #[test]
    fn narrowing_brings_the_size_under_the_cap() {
        // 3 x 3 = 9 before narrowing, above the cap of 4; 1 x 3 = 3 after.
        let limit = FakeLimit::new(&[("a", &["1", "2", "3"]), ("b", &["1", "2", "3"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "a", &["2"]);
        assert_eq!(
            ConfigurationSet::build(&limit, &c, 4).map(|s| s.len()),
            Ok(3)
        );
        assert_eq!(
            ConfigurationSet::build(&limit, &ConstraintsMap::new(), 4).map(|s| s.len()),
            Err(EngineError::ConfigurationSetTooLarge { max: 4 })
        );
    }

    #[test]
    fn repeated_option_ids_count_once() {
        let limit = FakeLimit::new(&[("a", &["x", "x", "y"])]);
        let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100).expect("valid");
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn limit_without_assumptions_has_one_empty_configuration() {
        let limit = FakeLimit::new(&[]);
        let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 1).expect("valid");
        assert_eq!(set.len(), 1);
        assert_eq!(set.iter().next().map(|c| c.iter().count()), Some(0));
    }

    #[test]
    fn later_allow_replaces_the_earlier_one() {
        let limit = FakeLimit::new(&[("a", &["x", "y"])]);
        let mut c = ConstraintsMap::new();
        allow(&mut c, "a", &["x"]);
        allow(&mut c, "a", &["y"]);
        let set = ConfigurationSet::build(&limit, &c, 100).expect("valid");
        assert_eq!(options_of(&set), vec![vec!["y"]]);
    }
}
