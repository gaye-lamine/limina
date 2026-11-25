//! Limina core: units, intervals, Limit trait and decision engine.
//!
//! The verdict is computed only by the deterministic engine of this crate.
//! The crate performs no I/O.

mod config;
mod constants;
mod dimension;
mod engine;
mod error;
mod interval;
mod limit;
mod limits;
mod quantity;
mod settings;
mod tolerance;
mod units;
mod units_parse;
mod verdict;

pub use config::{ConfigurationSet, ConstraintsMap};
pub use constants::{constant, PhysicalConstant, CODATA_VERSION, CONSTANTS};
pub use dimension::Dimension;
pub use engine::{bound_range, decide, EnclosureKind};
pub use error::EngineError;
pub use interval::Interval;
pub use limit::{
    Assumption, AssumptionId, AssumptionOption, Configuration, Direction, DomainRange, Limit,
    OptionId, ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain,
};
pub use limits::{Betz, Carnot, Cop};
pub use quantity::{AbsoluteTemperature, Measure, MeasureKind, Quantity, TemperatureDifference};
pub use settings::DecisionConfig;
pub use tolerance::{approx_eq, guard, ABS_TOL, REL_TOL};
pub use units::units_for_dimension;
pub use units_parse::{parse_unit, UnitInfo};
pub use verdict::{
    BoundRange, Claim, Decision, DecisionInput, EnclosureResult, Enclosures, Margin,
    MissingParameter, MissingStatus, SatisfyingConfig, Support, Verdict,
};
