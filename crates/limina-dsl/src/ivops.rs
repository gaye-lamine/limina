//! Interval evaluation of expressions, on the core `Interval`.
//!
//! Each occurrence of a parameter is evaluated on its own enclosure, so an
//! expression where a parameter appears more than once gets a range that can
//! be wider than its true range (`x - x` on `[1, 2]` is `[-1, 1]`). A wider
//! bound range can only move the engine away from IMPOSSIBLE; it never
//! creates a false IMPOSSIBLE. On point enclosures the evaluation is exact.

use std::collections::BTreeMap;

use limina_core::{EngineError, Interval};

use crate::expr::{BinOp, Cmp, Condition, Expr, Func};

/// Parameter enclosures, by name.
pub type Values = BTreeMap<String, Interval>;

fn neg(x: Interval) -> Result<Interval, EngineError> {
    Interval::new(-x.hi(), -x.lo())
}

fn powi(x: Interval, n: i32) -> Result<Interval, EngineError> {
    if n == 0 {
        return Interval::point(1.0);
    }
    if n < 0 {
        return Interval::point(1.0)?.div(powi(x, -n)?);
    }
    let (a, b) = (x.lo().powi(n), x.hi().powi(n));
    if n % 2 == 1 || x.lo() >= 0.0 {
        Interval::new(a.min(b), a.max(b))
    } else if x.hi() <= 0.0 {
        Interval::new(b, a)
    } else {
        Interval::new(0.0, a.max(b))
    }
}

fn sqrt(x: Interval) -> Result<Interval, EngineError> {
    if x.lo() < 0.0 {
        return Err(EngineError::NonFiniteValue {
            field: "sqrt of a negative value",
        });
    }
    Interval::new(x.lo().sqrt(), x.hi().sqrt())
}

fn ln(x: Interval) -> Result<Interval, EngineError> {
    if x.lo() <= 0.0 {
        return Err(EngineError::NonFiniteValue {
            field: "ln of a non-positive value",
        });
    }
    Interval::new(x.lo().ln(), x.hi().ln())
}

fn abs(x: Interval) -> Result<Interval, EngineError> {
    if x.lo() >= 0.0 {
        Ok(x)
    } else if x.hi() <= 0.0 {
        neg(x)
    } else {
        Interval::new(0.0, (-x.lo()).max(x.hi()))
    }
}

/// Interval value of `expr`.
pub fn eval(expr: &Expr, values: &Values) -> Result<Interval, EngineError> {
    match expr {
        Expr::Num(v) => Interval::point(*v),
        Expr::Const(c) => Interval::point(c.value()),
        Expr::Param(name) => values
            .get(name)
            .copied()
            .ok_or_else(|| EngineError::UnknownParameter(name.clone())),
        Expr::Neg(x) => neg(eval(x, values)?),
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, values)?, eval(b, values)?);
            match op {
                BinOp::Add => a.add(b),
                BinOp::Sub => a.sub(b),
                BinOp::Mul => a.mul(b),
                BinOp::Div => a.div(b),
            }
        }
        Expr::Pow(x, n) => powi(eval(x, values)?, *n),
        Expr::Call(f, args) => {
            let x = eval(&args[0], values)?;
            match f {
                Func::Sqrt => sqrt(x),
                Func::Ln => ln(x),
                Func::Exp => Interval::new(x.lo().exp(), x.hi().exp()),
                Func::Abs => abs(x),
                Func::Min | Func::Max => {
                    let y = eval(&args[1], values)?;
                    if *f == Func::Min {
                        Interval::new(x.lo().min(y.lo()), x.hi().min(y.hi()))
                    } else {
                        Interval::new(x.lo().max(y.lo()), x.hi().max(y.hi()))
                    }
                }
            }
        }
    }
}

/// Whether the condition holds for every value of the enclosures:
/// `a > b` holds when `a.lo > b.hi`, and so on. Anything less is a failure.
pub fn holds(c: &Condition, values: &Values) -> Result<bool, EngineError> {
    let (a, b) = (eval(&c.left, values)?, eval(&c.right, values)?);
    Ok(match c.cmp {
        Cmp::Gt => a.lo() > b.hi(),
        Cmp::Ge => a.lo() >= b.hi(),
        Cmp::Lt => a.hi() < b.lo(),
        Cmp::Le => a.hi() <= b.lo(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{parse_condition, parse_expression};

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::new(lo, hi).expect("interval")
    }

    fn run(src: &str, vals: &[(&str, Interval)]) -> Result<Interval, EngineError> {
        let names: Vec<String> = vals.iter().map(|(n, _)| n.to_string()).collect();
        let values: Values = vals.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        eval(&parse_expression(src, &names).expect("parse"), &values)
    }

    #[test]
    fn even_powers_are_exact_and_odd_powers_monotone() {
        assert_eq!(run("x^2", &[("x", iv(-2.0, 3.0))]), Ok(iv(0.0, 9.0)));
        assert_eq!(run("x^2", &[("x", iv(-3.0, -2.0))]), Ok(iv(4.0, 9.0)));
        assert_eq!(run("x^3", &[("x", iv(-2.0, 3.0))]), Ok(iv(-8.0, 27.0)));
        assert_eq!(run("x^-1", &[("x", iv(2.0, 4.0))]), Ok(iv(0.25, 0.5)));
        assert_eq!(
            run("x^-2", &[("x", iv(-1.0, 1.0))]),
            Err(EngineError::DivisionByIntervalContainingZero)
        );
    }

    #[test]
    fn functions_on_intervals() {
        assert_eq!(run("abs(x)", &[("x", iv(-3.0, 2.0))]), Ok(iv(0.0, 3.0)));
        assert_eq!(run("sqrt(x)", &[("x", iv(4.0, 9.0))]), Ok(iv(2.0, 3.0)));
        assert_eq!(run("min(x, 1)", &[("x", iv(0.0, 2.0))]), Ok(iv(0.0, 1.0)));
        assert_eq!(run("max(x, 1)", &[("x", iv(0.0, 2.0))]), Ok(iv(1.0, 2.0)));
        assert!(run("ln(x)", &[("x", iv(0.0, 1.0))]).is_err());
        assert!(run("sqrt(x)", &[("x", iv(-1.0, 1.0))]).is_err());
        assert_eq!(
            run("ln(exp(x))", &[("x", iv(1.0, 1.0))]).map(|i| (i.lo() - 1.0).abs() < 1e-15),
            Ok(true)
        );
    }

    #[test]
    fn a_repeated_variable_widens_the_range_conservatively() {
        assert_eq!(run("x - x", &[("x", iv(1.0, 2.0))]), Ok(iv(-1.0, 1.0)));
        assert_eq!(run("x - x", &[("x", iv(2.0, 2.0))]), Ok(iv(0.0, 0.0)));
    }

    #[test]
    fn conditions_hold_only_when_guaranteed() {
        let names = vec!["a".to_string(), "b".to_string()];
        let c = parse_condition("a > b", &names).expect("parse");
        let vals = |a: Interval, b: Interval| -> Values {
            [("a".to_string(), a), ("b".to_string(), b)].into()
        };
        assert_eq!(holds(&c, &vals(iv(5.0, 6.0), iv(1.0, 4.0))), Ok(true));
        assert_eq!(holds(&c, &vals(iv(3.0, 6.0), iv(1.0, 4.0))), Ok(false));
        assert_eq!(holds(&c, &vals(iv(4.0, 4.0), iv(4.0, 4.0))), Ok(false));
    }
}
