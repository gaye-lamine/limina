//! Prefixed and compound SI units (decision D-25).
//!
//! A unit expression is a product of factors. A factor is a unit symbol,
//! optionally with an SI prefix and an integer exponent (`^2`, `^-1`, `²`,
//! `⁻¹`), or a parenthesized expression. Factors are joined by `*`, `·`,
//! `.` or a space; `/` divides by the next factor or group only, so
//! `W/m^2/K` and `W/(m^2*K)` are the same unit. Matching is exact and
//! case-sensitive. Nothing is guessed: Celsius, Fahrenheit and any unknown
//! symbol are refused with `UnknownUnit`.
//!
//! Factors to SI are exact definitions (SI Brochure, 9th edition, 2019,
//! tables 2, 4, 7 and 8; IAU 2012 resolution B2 for the astronomical unit)
//! or come from exact CODATA 2022 constants (electronvolt).

use crate::dimension::Dimension;
use crate::error::EngineError;

/// A unit as the engine uses it: its dimension and its factor to SI base
/// units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitInfo {
    /// Dimension of the unit.
    pub dimension: Dimension,
    /// Multiply a value in this unit by this factor to get SI base units.
    pub factor_to_si: f64,
}

/// SI prefixes (SI Brochure 2019, table 7), longest first for matching.
const PREFIXES: [(&str, f64); 20] = [
    ("da", 1e1),
    ("Q", 1e30),
    ("R", 1e27),
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("d", 1e-1),
    ("c", 1e-2),
    ("m", 1e-3),
    ("µ", 1e-6),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

const fn d(e: [i8; 7]) -> Dimension {
    Dimension::new(e)
}

/// Exact value of the elementary charge in coulomb (CODATA 2022, exact).
const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;

/// Symbols that take an SI prefix: symbol, dimension, factor to SI.
const PREFIXABLE: [(&str, Dimension, f64); 30] = [
    ("m", d([1, 0, 0, 0, 0, 0, 0]), 1.0),
    ("g", d([0, 1, 0, 0, 0, 0, 0]), 1e-3),
    ("s", d([0, 0, 1, 0, 0, 0, 0]), 1.0),
    ("A", d([0, 0, 0, 1, 0, 0, 0]), 1.0),
    ("K", d([0, 0, 0, 0, 1, 0, 0]), 1.0),
    ("mol", d([0, 0, 0, 0, 0, 1, 0]), 1.0),
    ("cd", d([0, 0, 0, 0, 0, 0, 1]), 1.0),
    ("Hz", d([0, 0, -1, 0, 0, 0, 0]), 1.0),
    ("N", d([1, 1, -2, 0, 0, 0, 0]), 1.0),
    ("Pa", d([-1, 1, -2, 0, 0, 0, 0]), 1.0),
    ("J", d([2, 1, -2, 0, 0, 0, 0]), 1.0),
    ("W", d([2, 1, -3, 0, 0, 0, 0]), 1.0),
    ("C", d([0, 0, 1, 1, 0, 0, 0]), 1.0),
    ("V", d([2, 1, -3, -1, 0, 0, 0]), 1.0),
    ("F", d([-2, -1, 4, 2, 0, 0, 0]), 1.0),
    ("Ω", d([2, 1, -3, -2, 0, 0, 0]), 1.0),
    ("ohm", d([2, 1, -3, -2, 0, 0, 0]), 1.0),
    ("S", d([-2, -1, 3, 2, 0, 0, 0]), 1.0),
    ("Wb", d([2, 1, -2, -1, 0, 0, 0]), 1.0),
    ("T", d([0, 1, -2, -1, 0, 0, 0]), 1.0),
    ("H", d([2, 1, -2, -2, 0, 0, 0]), 1.0),
    ("lm", d([0, 0, 0, 0, 0, 0, 1]), 1.0),
    ("lx", d([-2, 0, 0, 0, 0, 0, 1]), 1.0),
    ("Bq", d([0, 0, -1, 0, 0, 0, 0]), 1.0),
    ("Gy", d([2, 0, -2, 0, 0, 0, 0]), 1.0),
    ("Sv", d([2, 0, -2, 0, 0, 0, 0]), 1.0),
    ("eV", d([2, 1, -2, 0, 0, 0, 0]), ELEMENTARY_CHARGE),
    ("Wh", d([2, 1, -2, 0, 0, 0, 0]), 3600.0),
    ("L", d([3, 0, 0, 0, 0, 0, 0]), 1e-3),
    ("bit", Dimension::DIMENSIONLESS, 1.0),
];

/// Symbols without prefix: symbol, dimension, factor to SI. Matched before
/// any prefix, so `min` is the minute and `h` alone is the hour.
const PLAIN: [(&str, Dimension, f64); 12] = [
    ("dimensionless", Dimension::DIMENSIONLESS, 1.0),
    ("rad", Dimension::DIMENSIONLESS, 1.0),
    ("sr", Dimension::DIMENSIONLESS, 1.0),
    ("min", d([0, 0, 1, 0, 0, 0, 0]), 60.0),
    ("h", d([0, 0, 1, 0, 0, 0, 0]), 3600.0),
    ("d", d([0, 0, 1, 0, 0, 0, 0]), 86_400.0),
    ("au", d([1, 0, 0, 0, 0, 0, 0]), 149_597_870_700.0),
    ("bar", d([-1, 1, -2, 0, 0, 0, 0]), 1e5),
    ("atm", d([-1, 1, -2, 0, 0, 0, 0]), 101_325.0),
    ("t", d([0, 1, 0, 0, 0, 0, 0]), 1e3),
    ("ha", d([2, 0, 0, 0, 0, 0, 0]), 1e4),
    ("%", Dimension::DIMENSIONLESS, 1e-2),
];

fn unknown(symbol: &str) -> EngineError {
    EngineError::UnknownUnit(symbol.to_string())
}

fn mul(a: UnitInfo, b: UnitInfo) -> UnitInfo {
    let (x, y) = (a.dimension.exponents(), b.dimension.exponents());
    let mut e = [0i8; 7];
    for i in 0..7 {
        e[i] = x[i].saturating_add(y[i]);
    }
    UnitInfo {
        dimension: Dimension::new(e),
        factor_to_si: a.factor_to_si * b.factor_to_si,
    }
}

fn pow(a: UnitInfo, n: i32) -> Result<UnitInfo, ()> {
    let mut e = [0i8; 7];
    for (i, x) in a.dimension.exponents().iter().enumerate() {
        e[i] = i8::try_from(i32::from(*x) * n).map_err(|_| ())?;
    }
    Ok(UnitInfo {
        dimension: Dimension::new(e),
        factor_to_si: a.factor_to_si.powi(n),
    })
}

/// One symbol without exponent: plain, else prefixable, else prefix +
/// prefixable. The kilogram is `kg` (prefix `k` on `g`).
fn atom(symbol: &str) -> Option<UnitInfo> {
    let info = |dimension, factor_to_si| UnitInfo {
        dimension,
        factor_to_si,
    };
    if let Some((_, dim, f)) = PLAIN.iter().find(|(s, _, _)| *s == symbol) {
        return Some(info(*dim, *f));
    }
    if let Some((_, dim, f)) = PREFIXABLE.iter().find(|(s, _, _)| *s == symbol) {
        return Some(info(*dim, *f));
    }
    for (p, pf) in PREFIXES {
        if let Some(rest) = symbol.strip_prefix(p) {
            if let Some((_, dim, f)) = PREFIXABLE.iter().find(|(s, _, _)| *s == rest) {
                return Some(info(*dim, *f * pf));
            }
        }
    }
    None
}

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0',
        '¹' => '1',
        '²' => '2',
        '³' => '3',
        '⁴' => '4',
        '⁵' => '5',
        '⁶' => '6',
        '⁷' => '7',
        '⁸' => '8',
        '⁹' => '9',
        '⁻' => '-',
        _ => return None,
    })
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    source: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_spaces(&mut self) -> bool {
        let start = self.pos;
        while self.peek() == Some(' ') {
            self.pos += 1;
        }
        self.pos > start
    }

    fn exponent(&mut self) -> Result<i32, EngineError> {
        let mut text = String::new();
        let caret = self.peek() == Some('^');
        if caret {
            self.pos += 1;
            if self.peek() == Some('(') {
                self.pos += 1;
                while let Some(c) = self.peek().filter(|c| *c != ')') {
                    text.push(c);
                    self.pos += 1;
                }
                if self.peek() != Some(')') {
                    return Err(unknown(self.source));
                }
                self.pos += 1;
            } else {
                if self.peek() == Some('-') {
                    text.push('-');
                    self.pos += 1;
                }
                while let Some(c) = self.peek().filter(char::is_ascii_digit) {
                    text.push(c);
                    self.pos += 1;
                }
            }
        } else {
            while let Some(c) = self.peek().and_then(superscript) {
                text.push(c);
                self.pos += 1;
            }
        }
        if text.is_empty() {
            return if caret {
                Err(unknown(self.source))
            } else {
                Ok(1)
            };
        }
        match text.parse::<i32>() {
            Ok(n) if n != 0 && (-12..=12).contains(&n) => Ok(n),
            _ => Err(unknown(self.source)),
        }
    }

    fn factor(&mut self) -> Result<UnitInfo, EngineError> {
        let base = if self.peek() == Some('(') {
            self.pos += 1;
            let inner = self.product()?;
            if self.peek() != Some(')') {
                return Err(unknown(self.source));
            }
            self.pos += 1;
            inner
        } else {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_alphabetic() || c == 'µ' || c == 'Ω' || c == '%' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let symbol: String = self.chars[start..self.pos].iter().collect();
            atom(&symbol).ok_or_else(|| unknown(self.source))?
        };
        let n = self.exponent()?;
        pow(base, n).map_err(|_| unknown(self.source))
    }

    fn product(&mut self) -> Result<UnitInfo, EngineError> {
        let mut acc = self.factor()?;
        loop {
            let spaced = self.skip_spaces();
            match self.peek() {
                Some('*') | Some('·') | Some('.') => {
                    self.pos += 1;
                    self.skip_spaces();
                    acc = mul(acc, self.factor()?);
                }
                Some('/') => {
                    self.pos += 1;
                    self.skip_spaces();
                    let f = pow(self.factor()?, -1).map_err(|_| unknown(self.source))?;
                    acc = mul(acc, f);
                }
                Some(c) if spaced && c != ')' => acc = mul(acc, self.factor()?),
                _ => return Ok(acc),
            }
        }
    }
}

/// Parses a unit expression.
///
/// Errors: `UnknownUnit` with the whole expression for any symbol outside
/// the SI tables above, a malformed exponent, an exponent outside -12..=12,
/// or unbalanced parentheses. A pure temperature (`K`, `mK`) is returned
/// with its dimension; `Quantity::from_unit` then refuses it with
/// `TemperatureKindRequired`, as for `K`.
pub fn parse_unit(symbol: &str) -> Result<UnitInfo, EngineError> {
    if symbol.is_empty() || symbol.trim() != symbol {
        return Err(unknown(symbol));
    }
    let mut parser = Parser {
        chars: symbol.chars().collect(),
        pos: 0,
        source: symbol,
    };
    let info = parser.product()?;
    if parser.pos != parser.chars.len()
        || !info.factor_to_si.is_finite()
        || info.factor_to_si <= 0.0
    {
        return Err(unknown(symbol));
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dim(s: &str) -> [i8; 7] {
        parse_unit(s).expect(s).dimension.exponents()
    }

    fn factor(s: &str) -> f64 {
        parse_unit(s).expect(s).factor_to_si
    }

    #[test]
    fn derived_units_have_their_si_dimension() {
        assert_eq!(dim("N"), [1, 1, -2, 0, 0, 0, 0]);
        assert_eq!(dim("Pa"), dim("N/m^2"));
        assert_eq!(dim("J"), dim("N*m"));
        assert_eq!(dim("W"), dim("J/s"));
        assert_eq!(dim("V"), dim("W/A"));
        assert_eq!(dim("Ω"), dim("V/A"));
        assert_eq!(dim("ohm"), dim("Ω"));
        assert_eq!(dim("F"), dim("C/V"));
        assert_eq!(dim("H"), dim("Wb/A"));
        assert_eq!(dim("T"), dim("Wb/m^2"));
        assert_eq!(dim("S"), dim("A/V"));
        assert_eq!(dim("Hz"), dim("s^-1"));
        assert_eq!(dim("lx"), dim("lm/m²"));
        assert_eq!(dim("eV"), dim("J"));
    }

    #[test]
    fn compound_forms_are_equivalent() {
        for (a, b) in [
            ("m/s^2", "m s⁻²"),
            ("W/(m^2*K)", "W/m^2/K"),
            ("W·m⁻²·K⁻¹", "W/(m² K)"),
            ("kWh/kg", "kWh kg^-1"),
            ("J mol^-1 K^-1", "J/(mol.K)"),
            ("bit/s", "bit s^-1"),
            ("kg/m^3", "kg m^(-3)"),
        ] {
            assert_eq!(parse_unit(a), parse_unit(b), "{a} vs {b}");
        }
    }

    #[test]
    fn prefixes_and_exact_factors() {
        assert_eq!(factor("km"), 1e3);
        assert_eq!(factor("kg"), 1.0);
        assert_eq!(factor("mg"), 1e-6);
        assert_eq!(factor("µm"), 1e-6);
        assert_eq!(factor("um"), 1e-6);
        assert_eq!(factor("nm"), 1e-9);
        assert_eq!(factor("GW"), 1e9);
        assert_eq!(factor("kWh"), 3.6e6);
        assert_eq!(factor("MeV"), 1e6 * ELEMENTARY_CHARGE);
        assert_eq!(factor("min"), 60.0);
        assert_eq!(factor("h"), 3600.0);
        assert_eq!(factor("hPa"), 100.0);
        assert_eq!(factor("km/h"), 1e3 / 3600.0);
        assert!(
            (factor("cm^3") - 1e-6).abs() <= 1e-21,
            "powers of prefixes are correct to one ulp"
        );
        assert_eq!(factor("mL"), 1e-6);
        assert_eq!(factor("au"), 149_597_870_700.0);
        assert_eq!(factor("Gbit/s"), 1e9);
        assert_eq!(factor("kWh/kg"), 3.6e6);
        assert_eq!(dim("kWh/kg"), [2, 0, -2, 0, 0, 0, 0]);
    }

    #[test]
    fn temperatures_inside_compound_units_are_differences() {
        assert_eq!(dim("W/(m^2*K^4)"), [0, 1, -3, 0, -4, 0, 0]);
        assert_eq!(dim("mK"), [0, 0, 0, 0, 1, 0, 0]);
    }

    #[test]
    fn nothing_is_guessed() {
        for s in [
            "", " W", "W ", "kw", "w", "°C", "degC", "°F", "W/", "m^", "m^0", "m^99", "(m", "m)",
            "xyz", "Kelvin", "mm/", "W//m", "1/s",
        ] {
            assert_eq!(
                parse_unit(s),
                Err(EngineError::UnknownUnit(s.to_string())),
                "{s:?}"
            );
        }
    }
}
