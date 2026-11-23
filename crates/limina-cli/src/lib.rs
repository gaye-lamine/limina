//! `limina check <file>`: a linter for physical performance claims.
//!
//! The file is read, its claims are extracted and evaluated through
//! `limina_api::analyze::analyze_text`, the same code as `POST /v1/analyze`,
//! and each claim is printed at its line and column. Every verdict comes
//! from the deterministic engine; this crate only formats it.

pub mod args;
pub mod check;
pub mod env;
pub mod position;
pub mod render;
pub mod run;
pub mod settle;

pub use args::{Command, Format};
pub use check::{check, LocatedClaim, Report};
pub use run::{run, Outcome};
