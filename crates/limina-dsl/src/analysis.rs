//! Static dimensional analysis, with the core rules for temperatures.
//!
//! - `+` and `-` need equal dimensions; `ln` and `exp` need dimensionless
//!   arguments; `sqrt` needs even exponents.
//! - Absolute minus absolute is a difference; absolute plus or minus a
//!   difference is absolute; absolute plus absolute is refused; a
//!   difference minus an absolute is refused.
//! - A number is dimensionless, so `T + 273.15` is refused: no silent Celsius.
//! - In a comparison, the literal `0` compares with any dimension.

use std::collections::BTreeMap;

use limina_core::Dimension;

use crate::error::DslError;
use crate::expr::{BinOp, Condition, Expr, Func};

/// Exponents of an absolute temperature or a temperature difference.
pub const TEMPERATURE: [i8; 7] = [0, 0, 0, 0, 1, 0, 0];

/// Static type of an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// A plain quantity of this dimension.
    Plain(Dimension),
    /// An absolute temperature.
    Absolute,
    /// A temperature difference.
    Difference,
}

impl Ty {
    /// SI exponents of the type.
    pub fn exponents(self) -> [i8; 7] {
        match self {
            Ty::Plain(d) => d.exponents(),
            Ty::Absolute | Ty::Difference => TEMPERATURE,
        }
    }

    fn is_temperature_kind(self) -> bool {
        matches!(self, Ty::Absolute | Ty::Difference)
    }
}

/// Types of the parameters, by name.
pub type Env = BTreeMap<String, Ty>;

fn combine(a: [i8; 7], b: [i8; 7], sign: i8, source: &str) -> Result<Dimension, DslError> {
    let mut out = [0i8; 7];
    for k in 0..7 {
        out[k] = b[k]
            .checked_mul(sign)
            .and_then(|v| a[k].checked_add(v))
            .ok_or_else(|| DslError::ExponentOverflow {
                source_text: source.to_string(),
            })?;
    }
    Ok(Dimension::new(out))
}

fn add_like(op: BinOp, l: Ty, r: Ty, source: &str) -> Result<Ty, DslError> {
    let name = if op == BinOp::Add {
        "addition"
    } else {
        "subtraction"
    };
    let mismatch = || DslError::TemperatureKindMismatch {
        source_text: source.to_string(),
        operation: name,
    };
    match (op, l, r) {
        (BinOp::Add, Ty::Absolute, Ty::Absolute) => Err(mismatch()),
        (BinOp::Sub, Ty::Absolute, Ty::Absolute) => Ok(Ty::Difference),
        (_, Ty::Absolute, Ty::Difference) => Ok(Ty::Absolute),
        (BinOp::Add, Ty::Difference, Ty::Absolute) => Ok(Ty::Absolute),
        (BinOp::Sub, Ty::Difference, Ty::Absolute) => Err(mismatch()),
        (_, Ty::Difference, Ty::Difference) => Ok(Ty::Difference),
        (_, Ty::Plain(a), Ty::Plain(b)) if a == b => Ok(Ty::Plain(a)),
        _ => Err(DslError::IncompatibleOperands {
            source_text: source.to_string(),
            operation: name,
            left: l.exponents(),
            right: r.exponents(),
        }),
    }
}

/// Static type of `expr`.
pub fn type_of(expr: &Expr, env: &Env, source: &str) -> Result<Ty, DslError> {
    let plain = |e: [i8; 7]| Ty::Plain(Dimension::new(e));
    Ok(match expr {
        Expr::Num(_) => Ty::Plain(Dimension::DIMENSIONLESS),
        Expr::Const(c) => plain(c.exponents()),
        Expr::Param(name) => *env.get(name).ok_or_else(|| DslError::UnknownSymbol {
            source_text: source.to_string(),
            name: name.clone(),
            position: 0,
        })?,
        Expr::Neg(x) => match type_of(x, env, source)? {
            Ty::Absolute => {
                return Err(DslError::TemperatureKindMismatch {
                    source_text: source.to_string(),
                    operation: "negation",
                })
            }
            t => t,
        },
        Expr::Bin(op @ (BinOp::Add | BinOp::Sub), a, b) => add_like(
            *op,
            type_of(a, env, source)?,
            type_of(b, env, source)?,
            source,
        )?,
        Expr::Bin(op, a, b) => {
            let (l, r) = (
                type_of(a, env, source)?.exponents(),
                type_of(b, env, source)?.exponents(),
            );
            let sign = if *op == BinOp::Mul { 1 } else { -1 };
            Ty::Plain(combine(l, r, sign, source)?)
        }
        Expr::Pow(x, n) => {
            let base = type_of(x, env, source)?.exponents();
            let factor = i8::try_from(*n).map_err(|_| DslError::ExponentOverflow {
                source_text: source.to_string(),
            })?;
            let mut out = [0i8; 7];
            for k in 0..7 {
                out[k] = base[k]
                    .checked_mul(factor)
                    .ok_or_else(|| DslError::ExponentOverflow {
                        source_text: source.to_string(),
                    })?;
            }
            plain(out)
        }
        Expr::Call(f, args) => {
            let first = type_of(&args[0], env, source)?;
            match f {
                Func::Ln | Func::Exp => {
                    if first != Ty::Plain(Dimension::DIMENSIONLESS) {
                        return Err(DslError::DimensionlessArgumentRequired {
                            source_text: source.to_string(),
                            function: f.name(),
                            found: first.exponents(),
                        });
                    }
                    Ty::Plain(Dimension::DIMENSIONLESS)
                }
                Func::Sqrt => {
                    let e = first.exponents();
                    if e.iter().any(|x| x % 2 != 0) {
                        return Err(DslError::OddExponentUnderSqrt {
                            source_text: source.to_string(),
                            found: e,
                        });
                    }
                    plain(e.map(|x| x / 2))
                }
                Func::Abs => first,
                Func::Min | Func::Max => {
                    let second = type_of(&args[1], env, source)?;
                    if first != second {
                        return Err(DslError::IncompatibleOperands {
                            source_text: source.to_string(),
                            operation: f.name(),
                            left: first.exponents(),
                            right: second.exponents(),
                        });
                    }
                    first
                }
            }
        }
    })
}

/// Checks that a formula has the declared dimension.
pub fn check_formula(
    expr: &Expr,
    env: &Env,
    source: &str,
    declared: Dimension,
    context: &str,
) -> Result<Ty, DslError> {
    let ty = type_of(expr, env, source)?;
    if ty.exponents() != declared.exponents() {
        return Err(DslError::DimensionMismatch {
            context: context.to_string(),
            computed: ty.exponents(),
            declared: declared.exponents(),
        });
    }
    Ok(ty)
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(v) if *v == 0.0)
}

/// Checks a comparison; returns the types of both sides.
pub fn check_condition(c: &Condition, env: &Env) -> Result<(Ty, Ty), DslError> {
    let l = type_of(&c.left, env, &c.text)?;
    let r = type_of(&c.right, env, &c.text)?;
    let ok = l == r || is_zero(&c.left) || is_zero(&c.right);
    if !ok {
        if l.is_temperature_kind() && r.is_temperature_kind() {
            return Err(DslError::TemperatureKindMismatch {
                source_text: c.text.clone(),
                operation: "comparison",
            });
        }
        return Err(DslError::IncompatibleOperands {
            source_text: c.text.clone(),
            operation: "comparison",
            left: l.exponents(),
            right: r.exponents(),
        });
    }
    Ok((l, r))
}

/// Whether a limit-level validity condition can map to a core error: the
/// core only has `InvalidTemperatureOrder`, so the condition must compare
/// absolute temperatures with each other or with 0.
pub fn is_temperature_order(c: &Condition, types: (Ty, Ty)) -> bool {
    let side = |e: &Expr, t: Ty| t == Ty::Absolute || is_zero(e);
    side(&c.left, types.0)
        && side(&c.right, types.1)
        && (types.0 == Ty::Absolute || types.1 == Ty::Absolute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{parse_condition, parse_expression};

    fn env() -> Env {
        let mut e = Env::new();
        e.insert("Th".into(), Ty::Absolute);
        e.insert("Tc".into(), Ty::Absolute);
        e.insert("T".into(), Ty::Absolute);
        e.insert("dT".into(), Ty::Difference);
        e
    }

    fn ty(src: &str) -> Result<Ty, DslError> {
        let names: Vec<String> = env().keys().cloned().collect();
        type_of(&parse_expression(src, &names)?, &env(), src)
    }

    #[test]
    fn temperature_kinds_follow_the_core_rules() {
        assert_eq!(ty("Th - Tc"), Ok(Ty::Difference));
        assert_eq!(ty("T + dT"), Ok(Ty::Absolute));
        assert_eq!(ty("dT + T"), Ok(Ty::Absolute));
        assert_eq!(ty("T - dT"), Ok(Ty::Absolute));
        assert!(matches!(
            ty("Th + Tc"),
            Err(DslError::TemperatureKindMismatch { .. })
        ));
        assert!(matches!(
            ty("dT - T"),
            Err(DslError::TemperatureKindMismatch { .. })
        ));
        assert!(matches!(
            ty("-T"),
            Err(DslError::TemperatureKindMismatch { .. })
        ));
    }

    #[test]
    fn no_silent_celsius() {
        assert!(matches!(
            ty("T + 273.15"),
            Err(DslError::IncompatibleOperands { .. })
        ));
        assert!(matches!(
            ty("T - 273.15"),
            Err(DslError::IncompatibleOperands { .. })
        ));
    }

    #[test]
    fn products_and_powers() {
        assert_eq!(ty("Tc/Th"), Ok(Ty::Plain(Dimension::DIMENSIONLESS)));
        assert_eq!(ty("T^2").map(Ty::exponents), Ok([0, 0, 0, 0, 2, 0, 0]));
        assert_eq!(ty("sqrt(T^2)").map(Ty::exponents), Ok(TEMPERATURE));
        assert_eq!(ty("h*c").map(Ty::exponents), Ok([3, 1, -2, 0, 0, 0, 0]));
        assert!(matches!(
            ty("T^64 * T^64"),
            Err(DslError::ExponentOverflow { .. })
        ));
    }

    #[test]
    fn conditions_and_the_zero_literal() {
        let names: Vec<String> = env().keys().cloned().collect();
        let c = parse_condition("Tc > 0", &names).expect("parse");
        let t = check_condition(&c, &env()).expect("typed");
        assert!(is_temperature_order(&c, t));
        let c = parse_condition("Th > Tc", &names).expect("parse");
        assert!(is_temperature_order(
            &c,
            check_condition(&c, &env()).expect("typed")
        ));
        let c = parse_condition("Th - Tc > 0", &names).expect("parse");
        assert!(!is_temperature_order(
            &c,
            check_condition(&c, &env()).expect("typed")
        ));
        let c = parse_condition("Th > 1", &names).expect("parse");
        assert!(check_condition(&c, &env()).is_err());
        let c = parse_condition("Th > dT", &names).expect("parse");
        assert!(matches!(
            check_condition(&c, &env()),
            Err(DslError::TemperatureKindMismatch { .. })
        ));
    }
}
