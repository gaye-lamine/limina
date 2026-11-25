//! Declarative physical limits.
//!
//! A limit is written as JSON with formulas in a small expression language.
//! Every formula and condition is checked for dimensional consistency when
//! the definition is compiled, then the limit runs in the existing decision
//! engine through the core `Limit` trait. The verdict is always computed by
//! `limina-core`. No language model, no network, no HTTP.
//!
//! Interval evaluation treats each occurrence of a parameter on its own, so
//! a formula with a repeated parameter gets a bound range that can be wider
//! than the true one. That is conservative: it can never create a false
//! IMPOSSIBLE. On point enclosures the evaluation is exact.

pub mod analysis;
pub mod compare;
pub mod curve;
pub mod definition;
pub mod equality;
pub mod error;
pub mod expr;
pub mod ivops;
pub mod limit;
pub mod registry;
pub mod report;
pub mod solve;
pub mod surface;

pub use compare::{
    compare_limits, relative_difference, Comparison, AGREEMENT_TOLERANCE, COMPARISON_SAMPLES,
    COMPARISON_SEED,
};
pub use curve::{
    bound_curve, bound_curve_enclosed, decisive, validity_zones, zones, CurveError, CurvePoint,
    MAX_CURVE_POINTS,
};
pub use definition::{Classification, EpistemicStatus, LimitDefinition, Status};
pub use equality::{combine_verdicts, EqualityDecision, EqualityLimit, EqualityOutcome};
pub use error::DslError;
pub use limit::{CompiledFormula, DeclarativeLimit};
pub use registry::{default_dir, Registry};
pub use report::{check_definition, check_limit, CheckReport, DimensionCheck, Monotonicity};
pub use solve::{solve_for, Solution, SolveError, SolveRequest, ThresholdDirection};
pub use surface::{bound_surface, SurfaceAxis, MAX_SURFACE_AXIS};
