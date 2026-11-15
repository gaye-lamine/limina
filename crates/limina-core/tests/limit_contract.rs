//! The shared `TestLimit` satisfies the `Limit` trait as the engine sees it.

mod common;

use common::{TestLimit, TEST_LIMIT_ID};
use limina_core::{Dimension, Direction, Limit, MeasureKind, ParameterId, ParameterSpec};

#[test]
fn test_limit_exposes_its_declarations_through_the_trait() {
    let limit = TestLimit::new(Direction::Min)
        .with_assumption("mode", &["a", "b"])
        .with_assumption("stage", &["one"])
        .with_parameter(ParameterSpec {
            id: ParameterId::new("p"),
            kind: MeasureKind::Plain(Dimension::DIMENSIONLESS),
            required: true,
            plausible: None,
        });
    let limit: &dyn Limit = &limit;

    assert_eq!(limit.id(), TEST_LIMIT_ID);
    assert_eq!(limit.direction(), Direction::Min);
    assert_eq!(
        limit.bounded_kind(),
        MeasureKind::Plain(Dimension::DIMENSIONLESS)
    );
    assert_eq!(limit.assumptions().len(), 2);
    assert_eq!(limit.assumptions()[0].domain.len(), 2);
    assert_eq!(limit.parameters()[0].id, ParameterId::new("p"));
}
