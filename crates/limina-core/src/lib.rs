//! Limina core: units, intervals, Limit trait and decision engine.
//!
//! The verdict is computed only by the deterministic engine of this crate.
//! The crate performs no I/O.

mod dimension;
mod error;
mod interval;
mod quantity;
mod settings;
mod tolerance;
mod units;

pub use dimension::Dimension;
pub use error::EngineError;
pub use interval::Interval;
pub use quantity::{AbsoluteTemperature, Measure, MeasureKind, Quantity, TemperatureDifference};
pub use settings::DecisionConfig;
pub use tolerance::{approx_eq, guard, ABS_TOL, REL_TOL};
