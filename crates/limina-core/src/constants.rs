//! Physical constants: one versioned table with value, unit, standard
//! uncertainty and source (decision D-25).
//!
//! Values are the CODATA 2022 recommended values, as published by NIST
//! ("Fundamental Physical Constants, Complete Listing, 2022 CODATA
//! adjustment", https://physics.nist.gov/cuu/Constants/Table/allascii.txt;
//! the file used is kept in `data/constants/codata-2022-allascii.txt`).
//! Constants exact since the 2019 SI redefinition have uncertainty 0. NIST
//! prints some exact derived constants truncated ("..."); those are computed
//! here from the defining constants, with the formula in their source field.
//! `g_n` is the conventional standard acceleration of gravity (exact by
//! definition, 3rd CGPM 1901), not a measured local value.

/// Version of the constants table.
pub const CODATA_VERSION: &str = "CODATA 2022";

const NIST: &str = "CODATA 2022, NIST allascii.txt";

/// One physical constant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalConstant {
    /// Symbol used in formulas, for example `hbar`.
    pub symbol: &'static str,
    /// Name, as in the CODATA listing.
    pub name: &'static str,
    /// Value in SI units.
    pub value: f64,
    /// Standard uncertainty in SI units; 0 for an exact constant.
    pub uncertainty: f64,
    /// SI unit, as written by CODATA.
    pub unit: &'static str,
    /// SI exponents, in the order L, M, T, I, Θ, N, J.
    pub exponents: [i8; 7],
    /// Where the value comes from.
    pub source: &'static str,
}

impl PhysicalConstant {
    /// Whether the constant is exact (uncertainty 0).
    pub fn is_exact(&self) -> bool {
        self.uncertainty == 0.0
    }
}

const PI: f64 = std::f64::consts::PI;
const H: f64 = 6.626_070_15e-34;
const KB: f64 = 1.380_649e-23;
const C: f64 = 299_792_458.0;
const E: f64 = 1.602_176_634e-19;
const NA: f64 = 6.022_140_76e23;
const HBAR: f64 = H / (2.0 * PI);
/// σ = 2 π⁵ kB⁴ / (15 h³ c²).
const SIGMA: f64 = 2.0 * PI * PI * PI * PI * PI * KB * KB * KB * KB / (15.0 * H * H * H * C * C);

const fn k(
    symbol: &'static str,
    name: &'static str,
    value: f64,
    uncertainty: f64,
    unit: &'static str,
    exponents: [i8; 7],
    source: &'static str,
) -> PhysicalConstant {
    PhysicalConstant {
        symbol,
        name,
        value,
        uncertainty,
        unit,
        exponents,
        source,
    }
}

/// The table, in a stable order.
pub const CONSTANTS: [PhysicalConstant; 24] = [
    k(
        "c",
        "speed of light in vacuum",
        C,
        0.0,
        "m s^-1",
        [1, 0, -1, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "h",
        "Planck constant",
        H,
        0.0,
        "J Hz^-1",
        [2, 1, -1, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "hbar",
        "reduced Planck constant",
        HBAR,
        0.0,
        "J s",
        [2, 1, -1, 0, 0, 0, 0],
        "CODATA 2022, exact: h / (2 pi)",
    ),
    k(
        "e",
        "elementary charge",
        E,
        0.0,
        "C",
        [0, 0, 1, 1, 0, 0, 0],
        NIST,
    ),
    k(
        "kB",
        "Boltzmann constant",
        KB,
        0.0,
        "J K^-1",
        [2, 1, -2, 0, -1, 0, 0],
        NIST,
    ),
    k(
        "NA",
        "Avogadro constant",
        NA,
        0.0,
        "mol^-1",
        [0, 0, 0, 0, 0, -1, 0],
        NIST,
    ),
    k(
        "R_gas",
        "molar gas constant",
        NA * KB,
        0.0,
        "J mol^-1 K^-1",
        [2, 1, -2, 0, -1, -1, 0],
        "CODATA 2022, exact: NA kB",
    ),
    k(
        "F_faraday",
        "Faraday constant",
        NA * E,
        0.0,
        "C mol^-1",
        [0, 0, 1, 1, 0, -1, 0],
        "CODATA 2022, exact: NA e",
    ),
    k(
        "sigma_SB",
        "Stefan-Boltzmann constant",
        SIGMA,
        0.0,
        "W m^-2 K^-4",
        [0, 1, -3, 0, -4, 0, 0],
        "CODATA 2022, exact: 2 pi^5 kB^4 / (15 h^3 c^2)",
    ),
    k(
        "b_wien",
        "Wien wavelength displacement law constant",
        2.897_771_955e-3,
        1e-12,
        "m K",
        [1, 0, 0, 0, 1, 0, 0],
        "CODATA 2022 (exact, printed truncated by NIST; the uncertainty bounds the truncation)",
    ),
    k(
        "Kcd",
        "luminous efficacy",
        683.0,
        0.0,
        "lm W^-1",
        [-2, -1, 3, 0, 0, 0, 1],
        NIST,
    ),
    k(
        "G",
        "Newtonian constant of gravitation",
        6.674_30e-11,
        0.000_15e-11,
        "m^3 kg^-1 s^-2",
        [3, -1, -2, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "eps0",
        "vacuum electric permittivity",
        8.854_187_818_8e-12,
        0.000_000_001_4e-12,
        "F m^-1",
        [-3, -1, 4, 2, 0, 0, 0],
        NIST,
    ),
    k(
        "mu0",
        "vacuum magnetic permeability",
        1.256_637_061_27e-6,
        0.000_000_000_20e-6,
        "N A^-2",
        [1, 1, -2, -2, 0, 0, 0],
        NIST,
    ),
    k(
        "alpha_fs",
        "fine-structure constant",
        7.297_352_564_3e-3,
        0.000_000_001_1e-3,
        "",
        [0, 0, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "m_e",
        "electron mass",
        9.109_383_713_9e-31,
        0.000_000_002_8e-31,
        "kg",
        [0, 1, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "m_p",
        "proton mass",
        1.672_621_925_95e-27,
        0.000_000_000_52e-27,
        "kg",
        [0, 1, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "m_n",
        "neutron mass",
        1.674_927_500_56e-27,
        0.000_000_000_85e-27,
        "kg",
        [0, 1, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "m_u",
        "atomic mass constant",
        1.660_539_068_92e-27,
        0.000_000_000_52e-27,
        "kg",
        [0, 1, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "a0",
        "Bohr radius",
        5.291_772_105_44e-11,
        0.000_000_000_82e-11,
        "m",
        [1, 0, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "R_inf",
        "Rydberg constant",
        10_973_731.568_157,
        0.000_012,
        "m^-1",
        [-1, 0, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "mu_B",
        "Bohr magneton",
        9.274_010_065_7e-24,
        0.000_000_002_9e-24,
        "J T^-1",
        [2, 0, 0, 1, 0, 0, 0],
        NIST,
    ),
    k(
        "sigma_T",
        "Thomson cross section",
        6.652_458_705_1e-29,
        0.000_000_006_2e-29,
        "m^2",
        [2, 0, 0, 0, 0, 0, 0],
        NIST,
    ),
    k(
        "g_n",
        "standard acceleration of gravity",
        9.806_65,
        0.0,
        "m s^-2",
        [1, 0, -2, 0, 0, 0, 0],
        NIST,
    ),
];

/// The constant with this symbol.
pub fn constant(symbol: &str) -> Option<&'static PhysicalConstant> {
    CONSTANTS.iter().find(|c| c.symbol == symbol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units_parse::parse_unit;

    fn rel(a: f64, b: f64) -> f64 {
        (a - b).abs() / b.abs()
    }

    #[test]
    fn symbols_are_unique() {
        for (i, a) in CONSTANTS.iter().enumerate() {
            for b in &CONSTANTS[i + 1..] {
                assert_ne!(a.symbol, b.symbol);
            }
        }
    }

    #[test]
    fn every_unit_has_the_declared_dimension() {
        for c in CONSTANTS {
            let unit = if c.unit.is_empty() {
                "dimensionless"
            } else {
                c.unit
            };
            let parsed = parse_unit(unit).unwrap_or_else(|e| panic!("{}: {e}", c.symbol));
            assert_eq!(parsed.dimension.exponents(), c.exponents, "{}", c.symbol);
            assert_eq!(parsed.factor_to_si, 1.0, "{} is in SI units", c.symbol);
        }
    }

    #[test]
    fn derived_exact_constants_match_the_printed_nist_digits() {
        // NIST prints these exact constants truncated; the computed value
        // must agree with every printed digit.
        for (symbol, printed) in [
            ("hbar", 1.054_571_817e-34),
            ("sigma_SB", 5.670_374_419e-8),
            ("R_gas", 8.314_462_618),
            ("F_faraday", 96_485.332_12),
        ] {
            let c = constant(symbol).expect(symbol);
            assert!(
                rel(c.value, printed) < 1e-9,
                "{symbol}: {} vs {printed}",
                c.value
            );
            assert!(c.is_exact());
        }
    }

    #[test]
    fn measured_constants_carry_their_uncertainty() {
        let g = constant("G").expect("G");
        assert!(!g.is_exact());
        assert!(rel(g.uncertainty, 1.5e-15) < 1e-12);
        assert!(constant("c").expect("c").is_exact());
        assert_eq!(constant("nope"), None);
        assert_eq!(CODATA_VERSION, "CODATA 2022");
    }

    #[test]
    fn consistency_relations_hold_within_the_uncertainties() {
        // mu0 eps0 c^2 = 1, both measured: within their combined uncertainty.
        let eps0 = constant("eps0").expect("eps0");
        let mu0 = constant("mu0").expect("mu0");
        let product = mu0.value * eps0.value * C * C;
        let u = (mu0.uncertainty / mu0.value).hypot(eps0.uncertainty / eps0.value);
        assert!((product - 1.0).abs() <= 3.0 * u, "{product}");
        // alpha = e^2 / (4 pi eps0 hbar c).
        let alpha = E * E / (4.0 * PI * eps0.value * HBAR * C);
        let fs = constant("alpha_fs").expect("alpha");
        assert!(
            (alpha - fs.value).abs()
                <= 3.0 * (fs.uncertainty + alpha * eps0.uncertainty / eps0.value)
        );
    }
}
