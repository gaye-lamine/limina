//! Output types of the decision engine (design 6.2).

/// `B_min` and `B_max` over `S` under one enclosure, in SI base units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundRange {
    /// Minimum of the selected bound values over `S`.
    pub b_min: f64,
    /// Maximum of the selected bound values over `S`.
    pub b_max: f64,
}
