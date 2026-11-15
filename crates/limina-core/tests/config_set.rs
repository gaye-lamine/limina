//! `ConfigurationSet` built from the shared `TestLimit`, and `TestLimit::bound`
//! on configurations of that set with a point enclosure.

mod common;

use std::collections::BTreeMap;

use common::TestLimit;
use limina_core::{
    ConfigurationSet, ConstraintsMap, Direction, Interval, Limit, ParameterEnclosure, ParameterId,
};

fn iv(lo: f64, hi: f64) -> Interval {
    Interval::new(lo, hi).expect("valid interval")
}

#[test]
fn test_limit_bound_on_a_point_enclosure() {
    let limit = TestLimit::new(Direction::Max)
        .with_assumption("mode", &["a", "b"])
        .with_bound(&["a"], 5.0, 5.0)
        .with_bound(&["b"], 8.0, 8.0);
    let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 10).expect("valid set");
    let bounds: Vec<Interval> = set
        .iter()
        .map(|c| limit.bound(c, &ParameterEnclosure::default()))
        .collect::<Result<_, _>>()
        .expect("every configuration has a bound");
    assert_eq!(bounds, vec![iv(5.0, 5.0), iv(8.0, 8.0)]);
}

#[test]
fn test_limit_multiplies_by_its_parameters() {
    let limit = TestLimit::new(Direction::Max)
        .with_assumption("mode", &["a"])
        .with_bound(&["a"], 5.0, 5.0)
        .with_parameter(limina_core::ParameterSpec {
            id: ParameterId::new("p"),
            kind: limina_core::MeasureKind::Plain(limina_core::Dimension::DIMENSIONLESS),
            required: true,
            plausible: None,
        });
    let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 10).expect("valid set");
    let mut values = BTreeMap::new();
    values.insert(ParameterId::new("p"), iv(3.0, 3.0));
    let params = ParameterEnclosure::new(values);
    let c = set.iter().next().expect("one configuration");
    assert_eq!(limit.bound(c, &params), Ok(iv(15.0, 15.0)));
}
