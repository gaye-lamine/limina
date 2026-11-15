//! Runtime physical dimension, as exponents of the 7 SI base dimensions.

/// Exponents of the 7 SI base dimensions, in the order
/// L (length), M (mass), T (time), I (electric current),
/// Θ (thermodynamic temperature), N (amount of substance), J (luminous intensity).
///
/// Two quantities are dimensionally compatible if and only if their
/// `Dimension` values are equal. The engine never converts between
/// different dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dimension([i8; 7]);

impl Dimension {
    /// The dimension of a pure number: all exponents are zero.
    pub const DIMENSIONLESS: Dimension = Dimension([0; 7]);

    /// Builds a dimension from its exponents, in the order L, M, T, I, Θ, N, J.
    pub const fn new(exponents: [i8; 7]) -> Dimension {
        Dimension(exponents)
    }

    /// Returns the exponents, in the order L, M, T, I, Θ, N, J.
    pub fn exponents(&self) -> [i8; 7] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Dimension;

    const POWER: [i8; 7] = [2, 1, -3, 0, 0, 0, 0];
    const ENERGY: [i8; 7] = [2, 1, -2, 0, 0, 0, 0];

    #[test]
    fn new_keeps_exponents() {
        assert_eq!(Dimension::new(POWER).exponents(), POWER);
    }

    #[test]
    fn dimensionless_has_all_zero_exponents() {
        assert_eq!(Dimension::DIMENSIONLESS.exponents(), [0; 7]);
        assert_eq!(Dimension::new([0; 7]), Dimension::DIMENSIONLESS);
    }

    #[test]
    fn equality_is_on_exponents() {
        assert_eq!(Dimension::new(POWER), Dimension::new(POWER));
        assert_ne!(Dimension::new(POWER), Dimension::new(ENERGY));
    }

    #[test]
    fn ordering_is_lexicographic_on_exponents() {
        let energy = Dimension::new(ENERGY);
        let power = Dimension::new(POWER);
        // Same L and M, T differs: -3 < -2.
        assert!(power < energy);
        let length = Dimension::new([1, 0, 0, 0, 0, 0, 0]);
        assert!(Dimension::DIMENSIONLESS < length);
        assert!(length < energy);
    }
}
