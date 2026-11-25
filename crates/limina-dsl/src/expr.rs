//! Expressions and conditions: lexer and recursive-descent parser.
//!
//! Grammar (nothing else is accepted):
//!
//! ```text
//! condition := expr ("<" | "<=" | ">" | ">=") expr
//! expr      := term (("+" | "-") term)*
//! term      := unary (("*" | "/") unary)*
//! unary     := "-" unary | power
//! power     := primary ("^" exponent)?
//! exponent  := integer | "-" integer | "(" "-"? integer ")"
//! primary   := number | name | function "(" expr ("," expr)* ")" | "(" expr ")"
//! function  := "sqrt" | "ln" | "exp" | "abs" | "min" | "max"
//! name      := a parameter, or one of kB h c e NA pi ln2
//! ```

use crate::error::DslError;

/// The grammar, as given to people and to the authoring prompt.
pub const GRAMMAR: &str = r#"condition := expr ("<" | "<=" | ">" | ">=") expr
expr      := term (("+" | "-") term)*
term      := unary (("*" | "/") unary)*
unary     := "-" unary | power
power     := primary ("^" exponent)?
exponent  := integer | "-" integer | "(" "-"? integer ")"
primary   := number | name | function "(" expr ("," expr)* ")" | "(" expr ")"
function  := "sqrt" | "ln" | "exp" | "abs" | "min" | "max"
name      := a parameter name, or a constant: kB h c e NA pi ln2, or a CODATA 2022
             constant of limina-core (hbar G eps0 mu0 m_e m_p m_n m_u alpha_fs
             sigma_SB R_gas F_faraday b_wien Kcd a0 R_inf mu_B g_n)
No assignment, no loop, no unit conversion function, no other name."#;

/// An exact SI constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constant {
    /// Boltzmann constant, 1.380649e-23 J/K.
    KB,
    /// Planck constant, 6.62607015e-34 J s.
    H,
    /// Speed of light, 299792458 m/s.
    C,
    /// Elementary charge, 1.602176634e-19 C.
    E,
    /// Avogadro constant, 6.02214076e23 1/mol.
    NA,
    /// pi, dimensionless.
    Pi,
    /// ln 2, dimensionless.
    Ln2,
    /// A constant of the CODATA table of `limina-core`, by index into
    /// `limina_core::CONSTANTS` (D-25). A measured constant is evaluated as
    /// the interval of its standard uncertainty.
    Physical(u8),
}

impl Constant {
    /// Every constant with its name.
    pub const ALL: [(&'static str, Constant); 7] = [
        ("kB", Constant::KB),
        ("h", Constant::H),
        ("c", Constant::C),
        ("e", Constant::E),
        ("NA", Constant::NA),
        ("pi", Constant::Pi),
        ("ln2", Constant::Ln2),
    ];

    /// Every constant: [`Constant::ALL`], then the CODATA constants of
    /// `limina-core` that are not already in it.
    pub fn all() -> Vec<(&'static str, Constant)> {
        let mut out: Vec<(&'static str, Constant)> = Constant::ALL.to_vec();
        for (i, c) in limina_core::CONSTANTS.iter().enumerate() {
            if !out.iter().any(|(n, _)| *n == c.symbol) {
                if let Ok(i) = u8::try_from(i) {
                    out.push((c.symbol, Constant::Physical(i)));
                }
            }
        }
        out
    }

    /// The constant with this name.
    pub fn from_name(name: &str) -> Option<Constant> {
        Constant::all()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| c)
    }

    fn physical(i: u8) -> Option<&'static limina_core::PhysicalConstant> {
        limina_core::CONSTANTS.get(usize::from(i))
    }

    /// Standard uncertainty in SI units; 0 for an exact constant.
    pub fn uncertainty(self) -> f64 {
        match self {
            Constant::Physical(i) => Constant::physical(i).map_or(0.0, |c| c.uncertainty),
            _ => 0.0,
        }
    }

    /// Exact SI value.
    pub fn value(self) -> f64 {
        match self {
            Constant::KB => 1.380649e-23,
            Constant::H => 6.62607015e-34,
            Constant::C => 299_792_458.0,
            Constant::E => 1.602176634e-19,
            Constant::NA => 6.02214076e23,
            Constant::Pi => std::f64::consts::PI,
            Constant::Ln2 => std::f64::consts::LN_2,
            Constant::Physical(i) => Constant::physical(i).map_or(f64::NAN, |c| c.value),
        }
    }

    /// SI exponents (L, M, T, I, Θ, N, J).
    pub fn exponents(self) -> [i8; 7] {
        match self {
            Constant::KB => [2, 1, -2, 0, -1, 0, 0],
            Constant::H => [2, 1, -1, 0, 0, 0, 0],
            Constant::C => [1, 0, -1, 0, 0, 0, 0],
            Constant::E => [0, 0, 1, 1, 0, 0, 0],
            Constant::NA => [0, 0, 0, 0, 0, -1, 0],
            Constant::Pi | Constant::Ln2 => [0; 7],
            Constant::Physical(i) => Constant::physical(i).map_or([0; 7], |c| c.exponents),
        }
    }
}

/// A function of the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// Square root.
    Sqrt,
    /// Natural logarithm.
    Ln,
    /// Exponential.
    Exp,
    /// Absolute value.
    Abs,
    /// Smaller of two values.
    Min,
    /// Larger of two values.
    Max,
}

impl Func {
    fn from_name(name: &str) -> Option<Func> {
        Some(match name {
            "sqrt" => Func::Sqrt,
            "ln" => Func::Ln,
            "exp" => Func::Exp,
            "abs" => Func::Abs,
            "min" => Func::Min,
            "max" => Func::Max,
            _ => return None,
        })
    }

    fn arity(self) -> usize {
        match self {
            Func::Min | Func::Max => 2,
            _ => 1,
        }
    }

    /// Name in the grammar.
    pub fn name(self) -> &'static str {
        match self {
            Func::Sqrt => "sqrt",
            Func::Ln => "ln",
            Func::Exp => "exp",
            Func::Abs => "abs",
            Func::Min => "min",
            Func::Max => "max",
        }
    }
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A decimal number, dimensionless.
    Num(f64),
    /// A parameter, by name.
    Param(String),
    /// An SI constant.
    Const(Constant),
    /// `-x`.
    Neg(Box<Expr>),
    /// `a op b`.
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// `x ^ n`, n an integer.
    Pow(Box<Expr>, i32),
    /// A function call.
    Call(Func, Vec<Expr>),
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
}

/// A condition: one comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// Left side.
    pub left: Expr,
    /// Operator.
    pub cmp: Cmp,
    /// Right side.
    pub right: Expr,
    /// The text, for messages.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    Cmp(Cmp),
    End,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    pos: usize,
}

fn parse_error(source: &str, position: usize, message: impl Into<String>) -> DslError {
    DslError::Parse {
        source_text: source.to_string(),
        position,
        message: message.into(),
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, DslError> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let pos = i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = chars[pos..i].iter().collect();
            let value: f64 = text
                .parse()
                .map_err(|_| parse_error(source, pos, format!("invalid number \"{text}\"")))?;
            out.push(Token {
                tok: Tok::Num(value),
                pos,
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(chars[pos..i].iter().collect()),
                pos,
            });
            continue;
        }
        let next = chars.get(i + 1).copied();
        let (tok, len) = match (c, next) {
            ('<', Some('=')) => (Tok::Cmp(Cmp::Le), 2),
            ('>', Some('=')) => (Tok::Cmp(Cmp::Ge), 2),
            ('<', _) => (Tok::Cmp(Cmp::Lt), 1),
            ('>', _) => (Tok::Cmp(Cmp::Gt), 1),
            ('+', _) => (Tok::Plus, 1),
            ('-', _) => (Tok::Minus, 1),
            ('*', _) => (Tok::Star, 1),
            ('/', _) => (Tok::Slash, 1),
            ('^', _) => (Tok::Caret, 1),
            ('(', _) => (Tok::LParen, 1),
            (')', _) => (Tok::RParen, 1),
            (',', _) => (Tok::Comma, 1),
            _ => {
                return Err(parse_error(
                    source,
                    pos,
                    format!("unexpected character '{c}'"),
                ))
            }
        };
        out.push(Token { tok, pos });
        i += len;
    }
    out.push(Token {
        tok: Tok::End,
        pos: chars.len(),
    });
    Ok(out)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    i: usize,
    params: &'a [String],
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        let last = self.tokens.len().saturating_sub(1);
        &self.tokens[self.i.min(last)]
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if self.i < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn error(&self, message: &str) -> DslError {
        parse_error(self.source, self.peek().pos, message)
    }

    fn expect(&mut self, tok: Tok, message: &str) -> Result<(), DslError> {
        if self.peek().tok == tok {
            self.bump();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn expr(&mut self) -> Result<Expr, DslError> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek().tok {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.term()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn term(&mut self) -> Result<Expr, DslError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek().tok {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.unary()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, DslError> {
        if self.peek().tok == Tok::Minus {
            self.bump();
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.power()
    }

    fn power(&mut self) -> Result<Expr, DslError> {
        let base = self.primary()?;
        if self.peek().tok != Tok::Caret {
            return Ok(base);
        }
        self.bump();
        let paren = self.peek().tok == Tok::LParen;
        if paren {
            self.bump();
        }
        let negative = self.peek().tok == Tok::Minus;
        if negative {
            self.bump();
        }
        let n = match self.peek().tok {
            Tok::Num(v) if v.fract() == 0.0 && v.abs() <= 64.0 => v as i32,
            _ => return Err(self.error("an exponent must be an integer between -64 and 64")),
        };
        self.bump();
        if paren {
            self.expect(Tok::RParen, "expected ')' after the exponent")?;
        }
        Ok(Expr::Pow(Box::new(base), if negative { -n } else { n }))
    }

    fn primary(&mut self) -> Result<Expr, DslError> {
        let token = self.bump();
        match token.tok {
            Tok::Num(v) => Ok(Expr::Num(v)),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen, "expected ')'")?;
                Ok(e)
            }
            Tok::Ident(name) => {
                if self.peek().tok == Tok::LParen {
                    let Some(func) = Func::from_name(&name) else {
                        return Err(DslError::UnknownSymbol {
                            source_text: self.source.to_string(),
                            name,
                            position: token.pos,
                        });
                    };
                    self.bump();
                    let mut args = vec![self.expr()?];
                    while self.peek().tok == Tok::Comma {
                        self.bump();
                        args.push(self.expr()?);
                    }
                    self.expect(Tok::RParen, "expected ')' after the arguments")?;
                    if args.len() != func.arity() {
                        return Err(parse_error(
                            self.source,
                            token.pos,
                            format!("{} takes {} argument(s)", func.name(), func.arity()),
                        ));
                    }
                    return Ok(Expr::Call(func, args));
                }
                if self.params.contains(&name) {
                    return Ok(Expr::Param(name));
                }
                if let Some(c) = Constant::from_name(&name) {
                    return Ok(Expr::Const(c));
                }
                Err(DslError::UnknownSymbol {
                    source_text: self.source.to_string(),
                    name,
                    position: token.pos,
                })
            }
            _ => Err(parse_error(
                self.source,
                token.pos,
                "expected a number, a name or '('",
            )),
        }
    }
}

/// Parses an expression over these parameter names.
pub fn parse_expression(source: &str, params: &[String]) -> Result<Expr, DslError> {
    let mut p = Parser {
        source,
        tokens: tokenize(source)?,
        i: 0,
        params,
    };
    let e = p.expr()?;
    if p.peek().tok != Tok::End {
        return Err(p.error("unexpected text after the expression"));
    }
    Ok(e)
}

/// Parses a condition over these parameter names.
pub fn parse_condition(source: &str, params: &[String]) -> Result<Condition, DslError> {
    let mut p = Parser {
        source,
        tokens: tokenize(source)?,
        i: 0,
        params,
    };
    let left = p.expr()?;
    let Tok::Cmp(cmp) = p.peek().tok.clone() else {
        return Err(p.error("expected a comparison <, <=, > or >="));
    };
    p.bump();
    let right = p.expr()?;
    if p.peek().tok != Tok::End {
        return Err(p.error("unexpected text after the condition"));
    }
    Ok(Condition {
        left,
        cmp,
        right,
        text: source.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn precedence_and_exponents() {
        let p = names(&["x"]);
        assert_eq!(
            parse_expression("-x^2", &p),
            Ok(Expr::Neg(Box::new(Expr::Pow(
                Box::new(Expr::Param("x".into())),
                2
            ))))
        );
        assert_eq!(
            parse_expression("x^-2", &p),
            Ok(Expr::Pow(Box::new(Expr::Param("x".into())), -2))
        );
        assert_eq!(parse_expression("x^(-2)", &p), parse_expression("x^-2", &p));
        assert!(matches!(
            parse_expression("1 - x/2*3", &p),
            Ok(Expr::Bin(BinOp::Sub, _, _))
        ));
        assert_eq!(parse_expression("1.5e3", &p), Ok(Expr::Num(1500.0)));
        assert_eq!(
            parse_expression("2*e", &p),
            Ok(Expr::Bin(
                BinOp::Mul,
                Box::new(Expr::Num(2.0)),
                Box::new(Expr::Const(Constant::E))
            ))
        );
    }

    #[test]
    fn errors_carry_their_position() {
        let p = names(&["Th", "Tc"]);
        let at = |src: &str| match parse_expression(src, &p) {
            Err(DslError::Parse { position, .. }) => position,
            other => panic!("expected a parse error for {src}, got {other:?}"),
        };
        assert_eq!(at("1 - "), 4);
        assert_eq!(at("(Th - Tc"), 8);
        assert_eq!(at("Th $ Tc"), 3);
        assert_eq!(at("Th ^ 1.5"), 5);
        assert_eq!(at("Th Tc"), 3);
        assert_eq!(at("min(Th)"), 0);
    }

    #[test]
    fn unknown_names_are_unknown_symbols_with_position() {
        let p = names(&["Th"]);
        assert_eq!(
            parse_expression("1 - Tx/Th", &p),
            Err(DslError::UnknownSymbol {
                source_text: "1 - Tx/Th".into(),
                name: "Tx".into(),
                position: 4
            })
        );
        assert!(matches!(
            parse_expression("log(Th)", &p),
            Err(DslError::UnknownSymbol { name, position: 0, .. }) if name == "log"
        ));
    }

    #[test]
    fn conditions() {
        let p = names(&["Th", "Tc"]);
        let c = parse_condition("Th > Tc", &p).expect("condition");
        assert_eq!(c.cmp, Cmp::Gt);
        assert!(matches!(
            parse_condition("Th", &p),
            Err(DslError::Parse { position: 2, .. })
        ));
        assert!(matches!(
            parse_condition("Th > Tc > 0", &p),
            Err(DslError::Parse { position: 8, .. })
        ));
        assert_eq!(parse_condition("Th >= 0", &p).map(|c| c.cmp), Ok(Cmp::Ge));
    }
}
