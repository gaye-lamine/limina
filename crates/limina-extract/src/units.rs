//! Scientific units that texts use and the core unit parser does not know,
//! converted to a unit it knows, with an exact factor and a visible note.
//!
//! `limina-core` is not changed: a value written in one of these units is
//! converted here, once, when the claim is read, and the engine receives a
//! core unit. Every factor is a definition or comes from the core constants
//! table (CODATA 2022):
//! - degree Celsius: `T_K = T_C + 273.15` (SI Brochure 2019, 2.3.1), for an
//!   absolute temperature only, never a temperature difference;
//! - bit rates: `Mbps` is the same unit as `Mbit/s`;
//! - angles: `deg = pi/180 rad`, `arcmin = deg/60`, `arcsec = deg/3600`;
//! - angstrom: `1 Å = 1e-10 m` (SI Brochure 2019, table 8);
//! - Julian year: `365.25 d = 31 557 600 s` (IAU);
//! - light-year: `c × Julian year = 9 460 730 472 580 800 m` (IAU);
//! - parsec: `648 000/pi au`, `au = 149 597 870 700 m` (IAU 2012 B2, 2015 B2);
//! - Hubble constant: `km/s/Mpc = 1e3 / (1e6 pc) s^-1`;
//! - solar mass: `GM_sun / G`, with the IAU 2015 B3 nominal
//!   `GM_sun = 1.327 124 4e20 m^3/s^2` and the CODATA 2022 `G` of the core table;
//! - solar luminosity: `3.828e26 W` (IAU 2015 B3 nominal);
//! - atomic mass unit: `u = Da = m_u` of the core table (CODATA 2022).

use std::f64::consts::PI;

/// IAU 2015 Resolution B3 nominal solar mass parameter, m^3/s^2.
pub const GM_SUN: f64 = 1.327_124_4e20;
/// IAU 2015 Resolution B3 nominal solar luminosity, W.
pub const L_SUN: f64 = 3.828e26;
/// Astronomical unit, m (IAU 2012 Resolution B2).
pub const AU: f64 = 149_597_870_700.0;
/// Julian year, s.
pub const JULIAN_YEAR: f64 = 31_557_600.0;
/// Speed of light, m/s (exact).
const C: f64 = 299_792_458.0;

/// Avoirdupois pound, kg (exact).
pub const POUND: f64 = 0.453_592_37;
/// Pound-force per square inch, Pa: 0.45359237 × 9.80665 N / 0.0254² m².
pub const PSI: f64 = 0.453_592_37 * 9.806_65 / (0.0254 * 0.0254);
/// Mechanical horsepower, W: 550 ft·lbf/s.
pub const HORSEPOWER: f64 = 550.0 * 0.3048 * 0.453_592_37 * 9.806_65;

/// Parsec, m.
pub fn parsec() -> f64 {
    AU * 648_000.0 / PI
}

/// A conversion to a unit the core parses: `value * factor + offset`, in `target`.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// Core unit of the converted value.
    pub target: &'static str,
    /// Multiplying factor.
    pub factor: f64,
    /// Added after the factor (only the degree Celsius has one).
    pub offset: f64,
    /// Whether the value is a temperature (absolute only).
    pub temperature: bool,
}

fn scale(target: &'static str, factor: f64) -> Conversion {
    Conversion {
        target,
        factor,
        offset: 0.0,
        temperature: false,
    }
}

fn core_constant(symbol: &str) -> Option<f64> {
    limina_core::constant(symbol).map(|c| c.value)
}

/// The conversion of a unit the core does not parse, written as texts write
/// it; None for any other unit (including every unit the core parses).
pub fn special(unit: &str) -> Option<Conversion> {
    let u = unit.trim();
    if u.is_empty() || limina_core::parse_unit(u).is_ok() {
        return None;
    }
    let deg = PI / 180.0;
    let lower = u.to_lowercase();
    Some(match u {
        "°C" | "℃" | "degC" | "deg C" => Conversion {
            target: "K",
            factor: 1.0,
            offset: 273.15,
            temperature: true,
        },
        "bps" => scale("bit/s", 1.0),
        "kbps" => scale("kbit/s", 1.0),
        "Mbps" => scale("Mbit/s", 1.0),
        "Gbps" => scale("Gbit/s", 1.0),
        "Tbps" => scale("Tbit/s", 1.0),
        "Å" | "\u{212B}" => scale("m", 1e-10),
        "M_sun" | "Msun" | "M☉" | "M_☉" | "M⊙" => scale("kg", GM_SUN / core_constant("G")?),
        "L_sun" | "Lsun" | "L☉" | "L_☉" | "L⊙" => scale("W", L_SUN),
        "yr" | "yrs" => scale("s", JULIAN_YEAR),
        "kyr" => scale("s", 1e3 * JULIAN_YEAR),
        "Myr" => scale("s", 1e6 * JULIAN_YEAR),
        "Gyr" => scale("s", 1e9 * JULIAN_YEAR),
        "ly" => scale("m", C * JULIAN_YEAR),
        "pc" => scale("m", parsec()),
        "kpc" => scale("m", 1e3 * parsec()),
        "Mpc" => scale("m", 1e6 * parsec()),
        "Gpc" => scale("m", 1e9 * parsec()),
        "km/s/Mpc" | "km s^-1 Mpc^-1" | "(km/s)/Mpc" => scale("Hz", 1e3 / (1e6 * parsec())),
        "u" | "Da" | "amu" => scale("kg", core_constant("m_u")?),
        "kDa" => scale("kg", 1e3 * core_constant("m_u")?),
        // US customary and other non-SI units, exact factors of NIST SP 811
        // (2008), appendix B. The gauss is written "gauss": "G" is the gravitational
        // constant and "Gs" the SI gigasecond.
        "°F" | "℉" | "degF" | "deg F" => Conversion {
            target: "K",
            factor: 5.0 / 9.0,
            offset: 273.15 - 32.0 * 5.0 / 9.0,
            temperature: true,
        },
        "psi" => scale("Pa", PSI),
        "mbar" => scale("Pa", 100.0),
        "Torr" | "torr" => scale("Pa", 101_325.0 / 760.0),
        "mmHg" => scale("Pa", 133.322_387_415),
        "hp" => scale("W", HORSEPOWER),
        "BTU" | "Btu" => scale("J", 1_055.055_852_62),
        "erg" => scale("J", 1e-7),
        "erg/s" => scale("W", 1e-7),
        "cal" => scale("J", 4.184),
        "kcal" => scale("J", 4_184.0),
        // "in" is read only as the unit a model gives, never after a number
        // of the text (see `known_in_text`): "3 in 10" is not 3 inches.
        "in" => scale("m", 0.0254),
        "ft" => scale("m", 0.3048),
        "yd" => scale("m", 0.9144),
        "mi" => scale("m", 1_609.344),
        "mph" => scale("m/s", 0.447_04),
        "kn" => scale("m/s", 1_852.0 / 3_600.0),
        "lb" | "lbs" => scale("kg", POUND),
        "oz" => scale("kg", POUND / 16.0),
        _ => match lower.as_str() {
            "inch" | "inches" => scale("m", 0.0254),
            "foot" | "feet" => scale("m", 0.3048),
            "yard" | "yards" => scale("m", 0.9144),
            "mile" | "miles" => scale("m", 1_609.344),
            "knot" | "knots" => scale("m/s", 1_852.0 / 3_600.0),
            "pound" | "pounds" => scale("kg", POUND),
            "ounce" | "ounces" => scale("kg", POUND / 16.0),
            "gauss" => scale("T", 1e-4),
            "horsepower" => scale("W", HORSEPOWER),
            "calorie" | "calories" => scale("J", 4.184),
            "kilocalorie" | "kilocalories" => scale("J", 4_184.0),
            "angstrom" | "angstroms" | "ångström" | "ångströms" => scale("m", 1e-10),
            "arcsec" | "arcsecond" | "arcseconds" => scale("rad", deg / 3600.0),
            "arcmin" | "arcminute" | "arcminutes" => scale("rad", deg / 60.0),
            "deg" | "degree" | "degrees" => scale("rad", deg),
            "solar mass" | "solar masses" => scale("kg", GM_SUN / core_constant("G")?),
            "solar luminosity" | "solar luminosities" => scale("W", L_SUN),
            "year" | "years" => scale("s", JULIAN_YEAR),
            "light-year" | "light-years" | "light year" | "light years" => {
                scale("m", C * JULIAN_YEAR)
            }
            "parsec" | "parsecs" => scale("m", parsec()),
            "megaparsec" | "megaparsecs" => scale("m", 1e6 * parsec()),
            _ => return None,
        },
    })
}

/// Whether a unit is understood: parsed by the core or converted here.
pub fn known(unit: &str) -> bool {
    limina_core::parse_unit(unit.trim()).is_ok() || special(unit).is_some()
}

/// Symbols that are ordinary English words after a number ("3 in 10"):
/// never read as a unit from the text itself, only when a model gives them.
const AMBIGUOUS_IN_TEXT: &[&str] = &["in"];

/// Whether a word written right after a number of the text is a unit.
pub fn known_in_text(unit: &str) -> bool {
    !AMBIGUOUS_IN_TEXT.contains(&unit.trim()) && known(unit)
}

/// The unit the core parses for `unit`: itself, or the target of its conversion.
pub fn core_unit(unit: &str) -> Option<String> {
    let u = unit.trim();
    if limina_core::parse_unit(u).is_ok() {
        return Some(u.to_string());
    }
    special(u).map(|c| c.target.to_string())
}

/// Words before or in a quotation that make a temperature a difference
/// ("a rise of 20 °C"): such a value is never converted as absolute.
const DIFFERENCE_WORDS: &[&str] = &[
    "difference",
    "rise",
    "rises",
    "drop",
    "drops",
    "increase",
    "increases",
    "decrease",
    "decreases",
    "delta",
    "gradient",
    "change",
    "warmer",
    "colder",
    "hotter",
    "cooler",
];

/// Whether `context` speaks of a temperature difference.
pub fn is_difference(context: &str) -> bool {
    if context.contains('Δ') {
        return true;
    }
    context
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| DIFFERENCE_WORDS.contains(&w))
}

/// `value` in `unit` converted to a core unit, with the note "1.5 solar
/// masses becomes 2.98e30 kg". None when the unit needs no conversion here,
/// or when it is a temperature written as a difference in `context`.
pub fn convert(value: f64, unit: &str, context: &str) -> Option<(f64, String, String)> {
    let c = special(unit)?;
    if c.temperature && is_difference(context) {
        return None;
    }
    let v = value * c.factor + c.offset;
    if !v.is_finite() {
        return None;
    }
    let note = format!(
        "{} {} becomes {} {}",
        crate::check::short_number(value),
        unit.trim(),
        crate::check::short_number(v),
        c.target
    );
    Some((v, c.target.to_string(), note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
    }

    #[test]
    fn each_unit_converts_with_its_defined_factor() {
        let cases: &[(&str, f64, &str, f64)] = &[
            ("°C", 27.0, "K", 300.15),
            ("Mbps", 10.0, "Mbit/s", 10.0),
            ("Gbps", 1.0, "Gbit/s", 1.0),
            ("kbps", 64.0, "kbit/s", 64.0),
            ("arcsec", 3600.0, "rad", PI / 180.0),
            ("arcmin", 60.0, "rad", PI / 180.0),
            ("deg", 180.0, "rad", PI),
            ("Å", 1.5, "m", 1.5e-10),
            ("angstrom", 2.0, "m", 2e-10),
            ("L_sun", 1.0, "W", 3.828e26),
            ("yr", 1.0, "s", 31_557_600.0),
            ("years", 2.0, "s", 63_115_200.0),
            ("ly", 1.0, "m", 9_460_730_472_580_800.0),
            ("light-years", 1.0, "m", 9_460_730_472_580_800.0),
            ("pc", 1.0, "m", 3.085_677_581_491_367e16),
            ("Mpc", 1.0, "m", 3.085_677_581_491_367e22),
            ("km/s/Mpc", 70.0, "Hz", 70e3 / 3.085_677_581_491_367e22),
        ];
        for (unit, v, target, expected) in cases {
            let (got, t, _) = convert(*v, unit, "").unwrap_or_else(|| panic!("{unit}"));
            assert_eq!(t, *target, "{unit}");
            assert!(close(got, *expected), "{unit}: {got} vs {expected}");
        }
        // Solar mass from IAU GM and CODATA 2022 G: 1.98841e30 kg.
        let (m, t, _) = convert(1.0, "solar masses", "").expect("solar mass");
        assert_eq!(t, "kg");
        assert!((m / 1.988_41e30 - 1.0).abs() < 1e-5, "{m}");
        assert_eq!(convert(1.0, "M☉", "").map(|x| x.0), Some(m));
        // Atomic mass unit: CODATA 2022 m_u.
        let (u, _, _) = convert(1.0, "u", "").expect("u");
        assert!(close(u, 1.660_539_068_92e-27), "{u}");
        assert_eq!(convert(1.0, "Da", "").map(|x| x.0), Some(u));
    }

    #[test]
    fn us_customary_and_nist_units_convert_with_their_exact_factors() {
        let cases: &[(&str, f64, &str, f64)] = &[
            ("°F", 212.0, "K", 373.15),
            ("°F", 32.0, "K", 273.15),
            ("psi", 1.0, "Pa", 6_894.757_293_168_361),
            ("mbar", 1013.25, "Pa", 101_325.0),
            ("torr", 760.0, "Pa", 101_325.0),
            ("mmHg", 1.0, "Pa", 133.322_387_415),
            ("hp", 1.0, "W", 745.699_871_582_270_2),
            ("BTU", 1.0, "J", 1_055.055_852_62),
            ("erg", 1e7, "J", 1.0),
            ("cal", 1.0, "J", 4.184),
            ("kcal", 1.0, "J", 4_184.0),
            ("inches", 1.0, "m", 0.0254),
            ("ft", 1.0, "m", 0.3048),
            ("feet", 10.0, "m", 3.048),
            ("yd", 1.0, "m", 0.9144),
            ("miles", 1.0, "m", 1_609.344),
            ("mph", 1.0, "m/s", 0.447_04),
            ("knots", 1.0, "m/s", 1_852.0 / 3_600.0),
            ("lb", 1.0, "kg", 0.453_592_37),
            ("pounds", 2.0, "kg", 0.907_184_74),
            ("oz", 16.0, "kg", 0.453_592_37),
            ("gauss", 1e4, "T", 1.0),
        ];
        for (unit, v, target, expected) in cases {
            let (got, t, note) = convert(*v, unit, "").unwrap_or_else(|| panic!("{unit}"));
            assert_eq!(t, *target, "{unit}");
            assert!(close(got, *expected), "{unit}: {got} vs {expected}");
            assert!(note.contains("becomes"), "{note}");
        }
        // A Fahrenheit difference is never converted as absolute.
        assert!(convert(20.0, "°F", "a rise of 20 °F").is_none());
        // "Gs" is the SI gigasecond for the core: never the gauss (written "gauss").
        assert!(special("Gs").is_none());
        // "in" is a unit only when a model gives it, never after a number of a text.
        assert!(known("in") && !known_in_text("in") && known_in_text("ft"));
    }

    #[test]
    fn a_celsius_difference_is_never_converted_as_absolute() {
        assert!(convert(20.0, "°C", "a temperature rise of 20 °C").is_none());
        assert!(convert(20.0, "°C", "ΔT = 20 °C").is_none());
        assert_eq!(
            convert(20.0, "°C", "in ambient air at 20 °C").map(|x| x.0),
            Some(293.15)
        );
    }

    #[test]
    fn core_units_and_unknown_symbols_are_not_converted_here() {
        for u in [
            "K", "m/s", "eV", "au", "bit/s", "furlong", "", "°X", "G", "dB",
        ] {
            assert!(special(u).is_none(), "{u}");
        }
        assert!(known("au") && known("solar masses") && !known("furlong"));
        assert_eq!(core_unit("arcsec").as_deref(), Some("rad"));
    }
}
