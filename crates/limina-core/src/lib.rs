//! Limina core: units, intervals, Limit trait and decision engine.
//!
//! The verdict is computed only by the deterministic engine of this crate.
//! The crate performs no I/O.

mod dimension;
mod error;
mod quantity;
mod units;

pub use dimension::Dimension;
pub use error::EngineError;
pub use quantity::{AbsoluteTemperature, Measure, MeasureKind, Quantity, TemperatureDifference};
