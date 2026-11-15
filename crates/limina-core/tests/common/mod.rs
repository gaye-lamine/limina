//! Shared helpers for the integration tests.
//!
//! `TestLimit` is a table of bound values chosen by the test. It is not a
//! physical limit and encodes no physics.

// Each test binary uses a different subset of these helpers.
#![allow(dead_code)]

pub mod scenario;

use std::collections::BTreeMap;

use limina_core::{
    Assumption, AssumptionId, AssumptionOption, Configuration, Dimension, Direction, EngineError,
    Interval, Limit, MeasureKind, OptionId, ParameterEnclosure, ParameterSpec,
};

/// Id returned by `TestLimit::id`.
pub const TEST_LIMIT_ID: &str = "test-limit";

/// A limit defined by a table: `B(c) = base(c) * p1 * p2 * ...` for the
/// declared parameters present in the enclosure.
pub struct TestLimit {
    pub direction: Direction,
    pub kind: MeasureKind,
    pub assumptions: Vec<Assumption>,
    pub parameters: Vec<ParameterSpec>,
    /// Base bound per configuration, keyed by the option ids of the
    /// configuration in assumption id order.
    pub bounds: BTreeMap<Vec<String>, Interval>,
}

impl TestLimit {
    /// Empty table, dimensionless bounded quantity.
    pub fn new(direction: Direction) -> TestLimit {
        TestLimit {
            direction,
            kind: MeasureKind::Plain(Dimension::DIMENSIONLESS),
            assumptions: Vec::new(),
            parameters: Vec::new(),
            bounds: BTreeMap::new(),
        }
    }

    /// Adds an assumption with the given option ids.
    pub fn with_assumption(mut self, id: &str, options: &[&str]) -> TestLimit {
        self.assumptions.push(Assumption {
            id: AssumptionId::new(id),
            domain: options
                .iter()
                .map(|o| AssumptionOption {
                    id: OptionId::new(*o),
                    value: None,
                })
                .collect(),
        });
        self
    }

    /// Sets `base(c) = [lo, hi]` for the configuration whose options, in
    /// assumption id order, are `options`.
    pub fn with_bound(mut self, options: &[&str], lo: f64, hi: f64) -> TestLimit {
        let key = options.iter().map(|o| o.to_string()).collect();
        let value = Interval::new(lo, hi).expect("test bound must be a valid interval");
        self.bounds.insert(key, value);
        self
    }

    /// Adds a parameter.
    pub fn with_parameter(mut self, spec: ParameterSpec) -> TestLimit {
        self.parameters.push(spec);
        self
    }
}

impl Limit for TestLimit {
    fn id(&self) -> &str {
        TEST_LIMIT_ID
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn bounded_kind(&self) -> MeasureKind {
        self.kind
    }

    fn assumptions(&self) -> &[Assumption] {
        &self.assumptions
    }

    fn parameters(&self) -> &[ParameterSpec] {
        &self.parameters
    }

    fn bound(
        &self,
        config: &Configuration,
        params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError> {
        let key: Vec<String> = config.iter().map(|(_, o)| o.as_str().to_string()).collect();
        let mut value =
            self.bounds
                .get(&key)
                .copied()
                .ok_or_else(|| EngineError::InvalidBound {
                    limit_id: TEST_LIMIT_ID.to_string(),
                })?;
        for spec in &self.parameters {
            if let Some(p) = params.get(&spec.id) {
                value = value.mul(p)?;
            }
        }
        Ok(value)
    }
}
