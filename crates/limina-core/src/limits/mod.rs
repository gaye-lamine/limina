//! Physical limits. One limit per file, each from its own spec.

mod betz;

pub use betz::Betz;
mod carnot;
pub use carnot::Carnot;
mod cop;
pub use cop::Cop;
