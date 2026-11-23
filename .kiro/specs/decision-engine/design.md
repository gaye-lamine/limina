# Design: decision-engine

Crate: `limina-core` only. Status: v5, frozen. Based on `requirements.md` v5. Signatures only, no bodies. No `tasks.md`, no code.

## 1. Scope and non-goals

In scope: the decision engine of R-01 to R-09. Out of scope: any physical limit (no formula, no constant), the extraction of quantities from text, the construction of the constraints map from text, any I/O.

The tests use a `TestLimit` (a table of bound values, no physics) defined in `tests/common/`. It is not part of the public API.

Dependency plan (D-14): `thiserror` as a dependency, `proptest` as a dev-dependency. Nothing else. The first task of `tasks.md` will be "remove `uom` from `crates/limina-core/Cargo.toml`". `tech.md` is edited by the author by hand.

## 2. Module layout

```
crates/limina-core/src/
  lib.rs          re-exports of the public API
  error.rs        EngineError
  tolerance.rs    REL_TOL, ABS_TOL, approx_eq, guard
  settings.rs     DecisionConfig
  dimension.rs    Dimension (exponents of the 7 SI base dimensions)
  units.rs        unit table (13 symbols) and lookup
  quantity.rs     Quantity, AbsoluteTemperature, TemperatureDifference, Measure, MeasureKind
  interval.rs     Interval (f64, SI, no directed rounding)
  limit.rs        Limit trait, Direction, Assumption, ParameterSpec, PlausibleDomain, ids
  config.rs       Configuration, ConstraintsMap, ConfigurationSet (S)
  verdict.rs      Verdict, MissingParameter, Support, SatisfyingConfig, Margin, BoundRange,
                  EnclosureResult, Enclosures, Decision
  engine.rs       decide, bound_range (pure functions, private rule helpers)
```

Each module stays small. All maps and sets are `BTreeMap` and `BTreeSet`, never `HashMap` (R-06.1: iteration order is part of determinism). No threads, no randomness, no clock, no `std::fs`, no `std::env`.

## 3. Dimension and units at the engine boundary

### 3.1 Decision: dynamic `Dimension`, no `uom` (D-14)

Quantities reach the engine at runtime, as a number plus a unit string. The engine therefore uses its own runtime `Dimension`: seven `i8` exponents (L, M, T, I, Θ, N, J), stored with the value in SI base units. The engine only compares dimensions and compares and divides SI numbers. The crate does not use `uom`, and the dependency is removed from the plan (see section 1).

### 3.2 Types

```rust
/// Exponents of the 7 SI base dimensions, in the order L, M, T, I, Θ, N, J.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dimension([i8; 7]);

impl Dimension {
    pub const DIMENSIONLESS: Dimension;
    pub const fn new(exponents: [i8; 7]) -> Dimension;
    pub fn exponents(&self) -> [i8; 7];
}

/// A value in SI base units with a runtime dimension.
/// Invariant: the dimension is never exactly temperature (Θ¹). See 3.3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity { /* si_value: f64, dimension: Dimension */ }

impl Quantity {
    /// Looks the unit up in the unit table. Unknown unit: EngineError::UnknownUnit.
    /// Unit "K": EngineError::TemperatureKindRequired.
    /// Non-finite value: EngineError::NonFiniteValue.
    pub fn from_unit(value: f64, unit: &str) -> Result<Quantity, EngineError>;
    pub fn from_si(value_si: f64, dimension: Dimension) -> Result<Quantity, EngineError>;
    pub fn to_unit(&self, unit: &str) -> Result<f64, EngineError>; // IncompatibleDimension if unit differs
    pub fn si_value(&self) -> f64;
    pub fn dimension(&self) -> Dimension;
}
```

### 3.3 Absolute temperature and temperature difference

They are distinct types. There is no Celsius or Fahrenheit unit in the table, so no silent conversion exists.

```rust
/// Absolute temperature in kelvin. Invariant: finite and >= 0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AbsoluteTemperature(f64);

/// Difference of two temperatures in kelvin. Invariant: finite. May be negative.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TemperatureDifference(f64);

impl AbsoluteTemperature {
    pub fn from_kelvin(k: f64) -> Result<AbsoluteTemperature, EngineError>; // NegativeAbsoluteTemperature
    pub fn kelvin(&self) -> f64;
}
impl TemperatureDifference {
    pub fn from_kelvin(k: f64) -> Result<TemperatureDifference, EngineError>;
    pub fn kelvin(&self) -> f64;
}
// Only these operations exist (as methods or std::ops impls):
//   AbsoluteTemperature - AbsoluteTemperature = TemperatureDifference
//   AbsoluteTemperature + TemperatureDifference = AbsoluteTemperature  (checked, error if < 0 K)
//   AbsoluteTemperature + AbsoluteTemperature  does not exist.

/// What the engine receives and compares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measure {
    Plain(Quantity),
    AbsoluteTemperature(AbsoluteTemperature),
    TemperatureDifference(TemperatureDifference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureKind {
    Plain(Dimension),
    AbsoluteTemperature,
    TemperatureDifference,
}

impl Measure {
    pub fn kind(&self) -> MeasureKind;
    pub fn si_value(&self) -> f64;
}
impl MeasureKind {
    /// Kind of the uncertainty of a value of this kind.
    /// AbsoluteTemperature -> TemperatureDifference; every other kind -> itself.
    pub fn uncertainty_kind(&self) -> MeasureKind;
}
```

Two measures are comparable only if their `MeasureKind` is equal. Otherwise the engine returns `IncompatibleDimension` (different dimensions) or `TemperatureKindMismatch` (absolute against difference). It never converts (R-01.2).

### 3.4 Unit table (exactly these 13 symbols)

| Symbol | Dimension (L,M,T,I,Θ,N,J) | Factor to SI |
|---|---|---|
| `W` | (2,1,-3,0,0,0,0) | 1 |
| `kW` | (2,1,-3,0,0,0,0) | 1e3 |
| `MW` | (2,1,-3,0,0,0,0) | 1e6 |
| `J` | (2,1,-2,0,0,0,0) | 1 |
| `kWh` | (2,1,-2,0,0,0,0) | 3.6e6 |
| `K` | (0,0,0,0,1,0,0) | 1 (only through the temperature types) |
| `m` | (1,0,0,0,0,0,0) | 1 |
| `s` | (0,0,1,0,0,0,0) | 1 |
| `kg` | (0,1,0,0,0,0,0) | 1 |
| `mol` | (0,0,0,0,0,1,0) | 1 |
| `A` | (0,0,0,1,0,0,0) | 1 |
| `cd` | (0,0,0,0,0,0,1) | 1 |
| `dimensionless` | (0,0,0,0,0,0,0) | 1 |

Lookup is an exact symbol match. No prefix parsing, no compound unit (`W/m^2`), no other unit. Anything else returns `UnknownUnit`.

```rust
pub(crate) struct UnitEntry { pub symbol: &'static str, pub dimension: Dimension, pub factor_to_si: f64 }
pub(crate) fn lookup_unit(symbol: &str) -> Result<UnitEntry, EngineError>;
```

## 4. Interval arithmetic and the guard

```rust
/// Closed interval [lo, hi] in SI base units. Invariant: finite, lo <= hi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval { /* lo: f64, hi: f64 */ }

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Result<Interval, EngineError>;   // NonFiniteValue, InvertedInterval
    pub fn point(x: f64) -> Result<Interval, EngineError>;
    pub fn lo(&self) -> f64;
    pub fn hi(&self) -> f64;
    pub fn add(self, other: Interval) -> Result<Interval, EngineError>;
    pub fn sub(self, other: Interval) -> Result<Interval, EngineError>;
    pub fn mul(self, other: Interval) -> Result<Interval, EngineError>;
    pub fn div(self, other: Interval) -> Result<Interval, EngineError>; // DivisionByIntervalContainingZero
    pub fn hull(self, other: Interval) -> Interval;
}
```

`add`, `sub`, `mul`, `div` serve limit implementors computing `B` over uncertain inputs (R-02.6). The engine itself uses `Interval` for the range of `B(c)` and for plausible ranges. Plausible ranges are kept as endpoints and are never enumerated (R-08.7).

### 4.1 No directed rounding, and the guard (D-15)

This version computes each endpoint with ordinary round-to-nearest `f64` operations. A computed endpoint can be off by a few ulps (about 2.2e-16 relative per operation), in either direction, so an `Interval` is not a guaranteed enclosure.

The engine handles this with a guard on the `IMPOSSIBLE` rules and on `C*` only:

- `g(x) = max(guard_rel * |x|, guard_abs)`, with the defaults `guard_rel = REL_TOL = 1e-9` and `guard_abs = ABS_TOL = 1e-12` (SI base units). These are many orders of magnitude above the accumulated rounding error of the short computations in this crate.
- Max-bound: `IMPOSSIBLE` iff `lo > B_max + g(B_max)`. Min-bound: `IMPOSSIBLE` iff `hi < B_min - g(B_min)`.
- `COMPATIBLE` stays exact: `hi <= B_min` (Max-bound), `lo >= B_max` (Min-bound). A rounding error there moves a verdict toward `CONDITIONAL` at worst, never toward a false `IMPOSSIBLE`. This is the stated rule, kept as written.
- `C*` uses the same guarded comparison (section 6.4, step 9).
- `REL_TOL` and `ABS_TOL` are also the comparison tolerance of the tests (`approx_eq`, P-05).
- **Deviation from D-01.** D-01 wrote `IMPOSSIBLE` as a strict exact inequality. The guard moves its boundary outward by `g`. Reason: a false `IMPOSSIBLE` is the worst error (`product.md`) and rounding can produce one. Cost: a claim that exceeds a bound by less than `g` is `CONDITIONAL`. With `guard_rel = guard_abs = 0` the exact rules are recovered (E-01).
- The guarantee stays "same input, same output" (P-10). Agreement with exact real arithmetic is guaranteed only outside the guard.

```rust
pub const REL_TOL: f64 = 1e-9;
pub const ABS_TOL: f64 = 1e-12; // SI base units
pub fn approx_eq(a: f64, b: f64) -> bool; // |a - b| <= ABS_TOL + REL_TOL * max(|a|, |b|)
/// g(x) = max(guard_rel * |x|, guard_abs)
pub fn guard(x: f64, guard_rel: f64, guard_abs: f64) -> f64;
```

Note: `x + g(x)` is non-decreasing in `x >= 0`. `x - g(x)` is non-decreasing in `x >= 0` because `guard_rel < 1` is enforced (D-18). The guard is global, in `DecisionConfig`; there is no per-limit guard (D-20).

## 5. Limit, configurations and parameters

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)] pub struct AssumptionId(String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)] pub struct OptionId(String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)] pub struct ParameterId(String);
// Each has: pub fn new(s: impl Into<String>) -> Self; pub fn as_str(&self) -> &str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Max, Min }

/// One allowed value of an assumption. Equality is on `id` only.
#[derive(Debug, Clone)]
pub struct AssumptionOption { pub id: OptionId, pub value: Option<Measure> }

/// A named assumption with a finite declared domain.
#[derive(Debug, Clone)]
pub struct Assumption { pub id: AssumptionId, pub domain: Vec<AssumptionOption> }

/// Plausible domain of a parameter, with its source (D-04).
/// Both forms are given in SI base units, in the kind of the parameter.
#[derive(Debug, Clone)]
pub enum DomainRange { Interval(Interval), Finite(Vec<f64>) }

#[derive(Debug, Clone)]
pub struct PlausibleDomain { pub range: DomainRange, pub source: String }

/// A parameter p of B. A `required` parameter that is not supplied is "missing".
#[derive(Debug, Clone)]
pub struct ParameterSpec {
    pub id: ParameterId,
    pub kind: MeasureKind,
    pub required: bool,
    pub plausible: Option<PlausibleDomain>, // None: no declared domain (R-05.5)
}

/// Values of parameters during one evaluation: a point interval for a supplied
/// parameter, the hull of the plausible domain for a missing one.
pub struct ParameterEnclosure { /* BTreeMap<ParameterId, Interval> */ }
impl ParameterEnclosure {
    pub fn get(&self, id: &ParameterId) -> Option<Interval>;
}

/// One assignment of an option to every assumption. Ordered (BTreeMap).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Configuration { /* BTreeMap<AssumptionId, OptionId> */ }
impl Configuration {
    pub fn option(&self, assumption: &AssumptionId) -> Option<&OptionId>;
    pub fn iter(&self) -> impl Iterator<Item = (&AssumptionId, &OptionId)>;
}

pub trait Limit {
    fn id(&self) -> &str;
    fn direction(&self) -> Direction;
    /// Kind of the bounded quantity; the claim must have the same kind.
    fn bounded_kind(&self) -> MeasureKind;
    fn assumptions(&self) -> &[Assumption];
    fn parameters(&self) -> &[ParameterSpec];
    /// Range of B(c, p) in SI base units when each parameter ranges over `params`.
    /// For a point enclosure the result is a point interval. The limit builds it
    /// with Interval arithmetic. Errors propagate to the caller.
    fn bound(&self, config: &Configuration, params: &ParameterEnclosure)
        -> Result<Interval, EngineError>;
}
```

A finite plausible set is used through its hull `[min, max]`. The hull is a superset of the set, so the permissive bound is not smaller and the restrictive bound is not larger than with the exact set. This widens toward `CONDITIONAL` and `INSUFFICIENT_DATA`, never toward a false `IMPOSSIBLE`.

### 5.1 Constraints and S (R-08, D-17)

```rust
/// Assumption id -> allowed option ids, derived upstream from the text.
#[derive(Debug, Clone, Default)]
pub struct ConstraintsMap { /* BTreeMap<AssumptionId, BTreeSet<OptionId>> */ }
impl ConstraintsMap {
    pub fn new() -> Self;
    pub fn allow(&mut self, assumption: AssumptionId, options: BTreeSet<OptionId>);
}

/// S: product of the domains narrowed by the constraints. Non-empty and at most
/// `max_configurations` long by construction.
#[derive(Debug, Clone)]
pub struct ConfigurationSet { /* Vec<Configuration> in lexicographic order */ }
impl ConfigurationSet {
    pub fn build(limit: &dyn Limit, constraints: &ConstraintsMap, max_configurations: usize)
        -> Result<ConfigurationSet, EngineError>;
        // UnknownAssumption, ConstraintOutsideDomain, EmptyConfigurationSet, ConfigurationSetTooLarge
    pub fn iter(&self) -> impl Iterator<Item = &Configuration>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool; // always false for a built set
}
```

An unconstrained assumption keeps its full domain. Order of checks in `build`: unknown assumption, constraint outside the declared domain, empty `S`, then the size cap. The size is the product of the narrowed domain sizes, computed with checked multiplication (an overflow counts as "too large") **before** any configuration is created. Only `S` after narrowing counts toward the cap.

## 6. Engine input, output and algorithm

### 6.1 Settings and input

```rust
/// Engine settings (R-02.3, D-15, D-17). Fields are private; validated on construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionConfig { /* k, guard_rel, guard_abs, max_configurations */ }

impl DecisionConfig {
    pub const DEFAULT_K: f64 = 2.0;
    pub const DEFAULT_MAX_CONFIGURATIONS: usize = 100_000;
    /// InvalidCoverageFactor if k is not finite or <= 0.
    /// InvalidGuard if guard_rel is not finite or not in 0 <= guard_rel < 1 (D-18),
    /// or if guard_abs is not finite or is negative.
    /// InvalidMaxConfigurations if max_configurations == 0.
    pub fn new(k: f64, guard_rel: f64, guard_abs: f64, max_configurations: usize)
        -> Result<DecisionConfig, EngineError>;
    pub fn k(&self) -> f64;
    pub fn guard_rel(&self) -> f64;
    pub fn guard_abs(&self) -> f64;
    pub fn max_configurations(&self) -> usize;
}
impl Default for DecisionConfig { /* k = 2, guard_rel = REL_TOL, guard_abs = ABS_TOL, cap = 100_000 */ }

#[derive(Debug, Clone)]
pub struct Claim { pub value: Measure, pub uncertainty: Option<Measure> }

#[derive(Debug, Clone)]
pub struct DecisionInput {
    pub claim: Claim,
    pub config: DecisionConfig,
    pub constraints: ConstraintsMap,
    pub known_parameters: BTreeMap<ParameterId, Measure>,
}
```

### 6.2 Output

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingStatus { NoDomain, HasDomain }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingParameter { pub id: ParameterId, pub status: MissingStatus }

/// `Depends` lists ALL missing parameters of the limit (D-13). Known simplification:
/// it may list parameters that B(c) does not actually depend on.
#[derive(Debug, Clone, PartialEq)]
pub enum Support { Certain, Depends(Vec<ParameterId>) }

#[derive(Debug, Clone, PartialEq)]
pub struct SatisfyingConfig { pub config: Configuration, pub support: Support }

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Impossible,
    Conditional { satisfying: Vec<SatisfyingConfig> }, // C*, never empty
    Compatible,
    InsufficientData { missing: Vec<MissingParameter> }, // ALL missing parameters, tagged (D-16)
}

/// Hull [min, max] of lo/B_max, lo/B_min, hi/B_max, hi/B_min (D-12). Unitless.
/// Always low <= high.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin { pub low: f64, pub high: f64 }

/// B_min and B_max over S under one enclosure, in SI base units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundRange { pub b_min: f64, pub b_max: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosureResult {
    pub bounds: BoundRange,
    pub margin: Option<Margin>, // None when B_min == 0 under this enclosure (D-12)
}

/// Both enclosures (D-11). Equal when no parameter is missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enclosures { pub permissive: EnclosureResult, pub restrictive: EnclosureResult }

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub verdict: Verdict,
    pub config: DecisionConfig,   // k, guard_rel, guard_abs, cap actually used
    pub claim_lo: f64,            // v - k*u, SI
    pub claim_hi: f64,            // v + k*u, SI
    /// None exactly when a missing parameter has no declared plausible domain (D-11, R-07.4).
    pub enclosures: Option<Enclosures>,
}
```

### 6.3 Entry points

```rust
pub fn decide(limit: &dyn Limit, input: &DecisionInput) -> Result<Decision, EngineError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnclosureKind { Permissive, Restrictive }

/// B_min and B_max over a set S, used by P-01 and by `decide`.
pub fn bound_range(limit: &dyn Limit, set: &ConfigurationSet, params: &ParameterEnclosure,
                   enclosure: EnclosureKind) -> Result<BoundRange, EngineError>;
```

### 6.4 Algorithm of `decide`

Notation: `g(x) = guard(x, guard_rel, guard_abs)`.

1. **Validate inputs.** `DecisionConfig` is already valid by construction. Claim kind equals `limit.bounded_kind()` (`IncompatibleDimension`, `TemperatureKindMismatch`). `u` has kind `claim.kind().uncertainty_kind()`. `v` and `u` finite, `u >= 0`. Compute `lo = v - k*u`, `hi = v + k*u` on the SI values (R-02). With no `u`, `u = 0`.
2. **Build S** with `ConfigurationSet::build(limit, constraints, max_configurations)` (R-08, D-07, D-08, D-17).
3. **Resolve parameters.** For each required parameter of the limit: if supplied, check its kind and use a point interval. If missing, add it to `missing` with `HasDomain` (the plausible domain exists, use its hull) or `NoDomain`. Supplied parameters that the limit does not declare give `UnknownParameter`.
4. **If any missing parameter is `NoDomain`:** return `Decision { verdict: InsufficientData { missing }, enclosures: None, .. }`. `missing` holds ALL missing parameters, each tagged `NoDomain` or `HasDomain` (D-16, D-22). No call to `Limit::bound` is made and no enclosure is computed (R-05.5, D-11, D-22).
5. **Per configuration `c`** in `S`, call `limit.bound(c, enclosure)` to get `[B_lo(c), B_hi(c)]`. Reject a negative or non-finite end with `InvalidBound` (R-02.5).
6. **Select the two enclosures.** Max-bound: permissive value of `c` is `B_hi(c)`, restrictive is `B_lo(c)`. Min-bound: permissive is `B_lo(c)`, restrictive is `B_hi(c)`. From each selection take `B_min` and `B_max` over `S`. These give `Enclosures` (equal when `missing` is empty).
7. **Apply the three rules once per enclosure** with that enclosure's values `b(c)` (private `rules_max` and `rules_min`), giving `verdict_perm` and `verdict_restr` as plain `Impossible | Conditional | Compatible`.
   - Max-bound: `Impossible` if `lo > b(c) + g(b(c))` for every `c`; else `Compatible` if `hi <= B_min`; else `Conditional`.
   - Min-bound: `Impossible` if `hi < b(c) - g(b(c))` for every `c`; else `Compatible` if `lo >= B_max`; else `Conditional`.
   - **Safety invariant (R-09).** `Impossible` comes from this explicit quantifier over `S`, not from a `B_max` shortcut. Because `guard_rel < 1` (D-18), it always equals the D-15 form `lo > B_max + g(B_max)` (Max-bound) or `hi < B_min - g(B_min)` (Min-bound), and a `debug_assert!` checks the agreement (unconditionally valid, D-18). The two outcomes are mutually exclusive because `g >= 0`.
8. **Combine.** If `verdict_perm == verdict_restr`, that is the verdict (D-03). Otherwise `InsufficientData { missing }` with all missing parameters tagged (all are `HasDomain` here). `Impossible` is therefore returned only when both enclosures give `Impossible`, so in particular under the permissive one (R-05.6). When `missing` is empty the two enclosures are identical, so the verdicts always agree.
9. **Build C\*** when the combined verdict is `Conditional` (D-10, D-15). Let `U(c)` be the permissive end of `B(c)` and `L(c)` the restrictive end.
   - Max-bound: keep `c` when `U(c) + g(U(c)) >= lo`. Tag `Certain` when `L(c) + g(L(c)) >= lo` (D-19).
   - Min-bound: keep `c` when `U(c) - g(U(c)) <= hi`, with `U` the permissive (lower) end. Tag `Certain` when the restrictive (upper) end `L(c)` satisfies `L(c) - g(L(c)) <= hi` (D-19).
   - A tag other than `Certain` is `Depends(all missing parameters)` (D-13). When `missing` is empty every member is `Certain`.
   - The configuration that attains `B_max` (Max-bound) or `B_min` (Min-bound) under the permissive values is always kept when the verdict is not `Impossible`, so `C*` is non-empty (E-04).
10. **Margins per enclosure (D-12).** If `B_min > 0`: `Some(Margin { low, high })` with `low` and `high` the min and max of the four ratios `lo/B_max`, `lo/B_min`, `hi/B_max`, `hi/B_min`. If `B_min == 0`: `None`, and no division happens. For `lo >= 0` the hull equals `[lo / B_max, hi / B_min]` (D-09).

The function is pure. It reads only its arguments.

## 7. Errors

```rust
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    #[error("incompatible dimensions: expected {expected:?}, found {found:?}")]
    IncompatibleDimension { expected: Dimension, found: Dimension },
    #[error("absolute temperature and temperature difference cannot be mixed")]
    TemperatureKindMismatch,
    #[error("unit K is ambiguous: use AbsoluteTemperature or TemperatureDifference")]
    TemperatureKindRequired,
    #[error("unknown unit: {0}")]
    UnknownUnit(String),
    #[error("absolute temperature below 0 K: {0}")]
    NegativeAbsoluteTemperature(f64),
    #[error("non-finite value for {field}")]
    NonFiniteValue { field: &'static str },
    #[error("negative uncertainty")]
    NegativeUncertainty,
    #[error("invalid coverage factor (must be finite and > 0)")]
    InvalidCoverageFactor,
    #[error("invalid guard (guard_rel must be finite and 0 <= guard_rel < 1; guard_abs must be finite and >= 0)")]
    InvalidGuard,
    #[error("invalid max_configurations (must be >= 1)")]
    InvalidMaxConfigurations,
    #[error("invalid bound for limit {limit_id}: negative or non-finite")]
    InvalidBound { limit_id: String },
    #[error("inverted interval")]
    InvertedInterval,
    #[error("division by an interval containing zero")]
    DivisionByIntervalContainingZero,
    #[error("unknown assumption: {0}")]
    UnknownAssumption(String),
    #[error("constraint on {assumption} names option {option} outside its declared domain")]
    ConstraintOutsideDomain { assumption: String, option: String },
    #[error("unknown parameter: {0}")]
    UnknownParameter(String),
    #[error("empty configuration set")]
    EmptyConfigurationSet,
    #[error("configuration set larger than the cap of {max}")]
    ConfigurationSetTooLarge { max: usize },
}
```

No `unwrap()` in library code. `f64` fields make the enum `PartialEq` only, which is enough for tests.

## 8. Traceability

| Item | Satisfied by |
|---|---|
| R-01.1 | `Quantity`, `Measure`, `AbsoluteTemperature`, `TemperatureDifference` (3.2, 3.3) |
| R-01.2 | `MeasureKind` equality check, `IncompatibleDimension`, `TemperatureKindMismatch`, no conversion API (3.3) |
| R-01.3 | `decide` step 1 against `Limit::bounded_kind` |
| R-01.4 | `decide` step 1, `uncertainty_kind` |
| R-01.5 | `AbsoluteTemperature` in kelvin only, no Celsius unit (3.3, 3.4) |
| R-02.1, R-02.2 | `decide` step 1 (`lo`, `hi`, `u = 0` when absent) |
| R-02.3 | `DecisionConfig`, `DecisionConfig::default` (6.1) |
| R-02.4 | `DecisionConfig::new` (`InvalidCoverageFactor`, `InvalidGuard` for `guard_rel` outside `[0, 1)` or negative or non-finite `guard_abs`, `InvalidMaxConfigurations`), `decide` step 1 (`NonFiniteValue`, `NegativeUncertainty`) |
| R-02.5 | `decide` step 5, `InvalidBound` |
| R-02.6 | `Interval` ops, `Limit::bound` (4) |
| R-03 | `decide` steps 7 and 9 (`rules_max`, guarded `Impossible`, exact `Compatible`, guarded `C*`) |
| R-04 | `decide` steps 7 and 9 (`rules_min`) |
| R-05.1 to R-05.3 | `decide` steps 3, 6, 7, 8 |
| R-05.4 | `decide` step 8, `MissingParameter` |
| R-05.5 | `decide` step 4, `MissingStatus::NoDomain` |
| R-05.6 | `decide` steps 7 and 8 |
| R-05.7, R-05.8 | `decide` step 9 (guarded `Certain` test on the restrictive end, D-19), `Support` |
| R-05.9 | no guessing of domains: only `ParameterSpec::plausible`; `Limit` is the only source |
| R-06.1 | `BTreeMap`/`BTreeSet`, pure `decide`, no hidden state (2) |
| R-06.2, R-06.3 | no LLM or I/O dependency; the crate depends only on `thiserror` |
| R-06.4 | `EngineError` with `thiserror`, no `unwrap()` |
| R-07.1 | `Decision`, `DecisionConfig`, `Enclosures`, `EnclosureResult` |
| R-07.2 | `Verdict::Conditional { satisfying }`, `Support` |
| R-07.3 | `Verdict::InsufficientData { missing }`, `MissingStatus` |
| R-07.4 | `Decision::enclosures == None` in `decide` step 4 |
| R-07.5 | no free-text output in this crate; `EngineError` messages and docs avoid the listed words (checked by a test, 9.4) |
| R-08.1 to R-08.5 | `ConfigurationSet::build`, `ConstraintsMap` (5.1) |
| R-08.6 | `ConfigurationSet::build` size check before enumeration, `ConfigurationSetTooLarge` |
| R-08.7 | `DomainRange` kept as endpoints; the enclosure uses the hull only (5) |
| R-09 | `decide` step 7 (explicit quantifier with guard), P-02 |
| D-01 | `rules_max`, `rules_min` on `[lo, hi]`; deviation recorded in 4.1 |
| D-02 | `rules_min` |
| D-03 | `decide` step 8 |
| D-04 | `ParameterSpec::plausible`, `decide` steps 3 and 4 |
| D-05 | no P-07 to P-09 in this crate; noted in the test module doc |
| D-06 | `REL_TOL`, `ABS_TOL`, `approx_eq` (4.1) |
| D-07 | `EngineError::EmptyConfigurationSet` |
| D-08 | `ConfigurationSet::build` |
| D-09 | superseded in form by D-12 (equal for `lo >= 0`) |
| D-10 | `decide` step 9, `Support` |
| D-11 | `Enclosures`, `Decision::enclosures`, `decide` steps 4 and 6 |
| D-12 | `Margin`, `decide` step 10 |
| D-13 | `Support::Depends` lists all missing parameters (6.2) |
| D-14 | section 1 (dependency plan), section 3.1; first task of `tasks.md` removes `uom` |
| D-15 | `DecisionConfig`, `guard`, `decide` steps 7 and 9, section 4.1 |
| D-16 | `MissingParameter`, `MissingStatus`, `decide` steps 4 and 8 |
| D-17 | `DecisionConfig::max_configurations`, `ConfigurationSet::build`, `ConfigurationSetTooLarge` |
| D-18 | `DecisionConfig::new` (`InvalidGuard`), 4.1 note, `decide` step 7 `debug_assert!`, example tests (9.4), extreme-guard generator mode (9.2) |
| D-19 | `decide` step 9 (`Certain` iff guarded comparison on the restrictive end, else `Depends(all missing)`), example tests (9.4) |
| D-20 | `DecisionConfig` holds the only guard; `Limit` has no guard method (5, 6.1) |
| D-21 | `DecisionConfig::new` (`InvalidGuard`, `InvalidMaxConfigurations`), example tests (9.4) |
| D-22 | `decide` step 4 (`enclosures: None`, `missing` tagged), `Decision::enclosures`, example tests (9.4) |
| P-01 | `bound_range` over nested sets |
| P-02 | `decide` step 7 quantifier with guard |
| P-03 | `Verdict` is an enum; the rules are a total partition for `g >= 0` |
| P-04 | unit table (3.4), SI normalisation at the boundary |
| P-05 | `Quantity::from_unit`, `Quantity::to_unit`, `approx_eq` |
| P-06 | `IncompatibleDimension`, `TemperatureKindMismatch` |
| P-10 | determinism rules in section 2, `PartialEq` on `Decision` |
| E-01 | `rules_max`, `rules_min` with `u = 0` and `guard_rel = guard_abs = 0` |
| E-02 | monotone dependence of `lo`, `hi` on `u`; the guard depends only on `B` |
| E-03 | `ConfigurationSet::build` |
| E-04 | `decide` step 9 (the configuration attaining the permissive extreme is kept) |

## 9. Testing strategy

All property tests live in `crates/limina-core/tests/` and use `proptest` 1.12.0.

### 9.1 Fixed seeds, regressions, readable counterexamples

- **Fixed seed.** Tests do not use the default `proptest!` seeding. A helper `tests/common/mod.rs::runner() -> TestRunner` builds `TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &SEED))` with a constant 32-byte `SEED` and a fixed `cases` count (256 locally, 1024 in CI via a constant). To be confirmed against the proptest 1.12.0 docs when implementing.
- **Regressions.** The failure persistence file (`proptest-regressions/`) is committed. Each failing case found is also copied into a named `#[test]` in `tests/regressions.rs` with the shrunk values written by hand.
- **Readable counterexamples.** The strategies produce a `Scenario` struct instead of loose tuples. `Scenario` has a hand-written `Debug` that prints, for example: `Max-bound | B(c1)=5, B(c2)=8 | claim 7.5 +- 0.5 (k=2) | guard 1e-9/1e-12 | lo=6.5 hi=8.5 | missing: [p1 in [1,2]]`. `proptest` prints the `Debug` of the shrunk input, so the failure is readable without decoding.

### 9.2 Shared generators (`tests/common/`)

- `TestLimit`: a table `BTreeMap<Configuration, Interval>` of `B(c)` values and a list of parameters. Where a parameter is present, `B(c, p) = base(c) * p` through `Interval::mul`. This is an arbitrary test function, not physics.
- **Bound values** `base(c)`: integers `0..=1_000` converted to `f64`. Integers keep comparisons exact in the guard-free case.
- **Configurations**: 1 to 5 options on 1 to 3 assumptions.
- **Guard settings**: four generated modes: `guard = 0` (exact rules), default guard, a large guard (`guard_rel` in `[1e-3, 0.5]`) so the guard effect is visible and not hidden by rounding, and an extreme guard (`guard_rel` in `[0.9, 0.999]`) that exercises the agreement between the quantifier form and the `B_max` / `B_min` form of D-15.
- **Claim** `v`: `0..=2_000` integers, plus an offset strategy relative to a chosen bound, so boundary values (`offset` in `-2..=2`) are hit often. With a guard, boundary values are generated at `b + g(b)` (or `b - g(b)`) and at one ulp on each side (`f64::next_up`, `f64::next_down`).
- **Uncertainty** `u`: `0..=50` integers and `k` in `{1, 2, 3}`, so `k*u` is an exact integer. A separate real-valued strategy (`f64` in range, away from boundaries by more than the guard) covers non-integer cases.
- **Shrinking.** Plain range strategies shrink toward 0, `vec` shrinks toward fewer configurations, and `prop_map` is preferred over `prop_flat_map` (which shrinks poorly). Claim values are generated as `bound + offset` through `prop_map`, so shrinking reduces the offset toward 0 and keeps the boundary visible.
- **Oracle.** The test oracle computes the guarded boundaries with its own inline formula, `b + f64::max(rel * b.abs(), abs)` and `b - f64::max(rel * b.abs(), abs)`, not by calling the crate's `guard` function, so the oracle does not share code with the engine.

### 9.3 Properties

| Id | Generator and assertion |
|---|---|
| P-01 | Generate a `TestLimit` and a constraint map `narrow`, then a `wide` map that allows a superset of options per assumption (built by adding options). Assert `bound_range(wide).b_max >= bound_range(narrow).b_max` and `bound_range(wide).b_min <= bound_range(narrow).b_min`, for both enclosure kinds. Shrinks toward fewer and smaller bounds. |
| P-02 | Direction in `{Max, Min}`, all three guard modes. Choose a configuration `c` in `S`. Generate `v <= B(c)` (Max-bound; `v >= B(c)` for Min-bound), and also `lo` within the guard: `lo` in `[B(c), B(c) + g(B(c))]` (Max-bound), `hi` in `[B(c) - g(B(c)), B(c)]` (Min-bound). Any `u >= 0` and `k`. Assert `verdict != Impossible`. Variant with missing parameters: `B(c)` taken at the permissive end. Verifies R-09. |
| P-03 (a) | No missing parameters. For all `v`, `u`, `k`, `S` and the four guard modes: an independent oracle computes the three predicates from the raw numbers with the guarded formulas of R-03 and R-04. Assert exactly one of the three outcomes holds (`!(imp && comp)` and `cond == !imp && !comp`), and that `decide` returns the matching variant. Boundary offsets are generated on purpose. |
| P-03 (b) | Any valid input, with or without missing parameters: `decide` returns `Ok` with exactly one `Verdict` variant, never panics. Payloads are checked: `Margin.low <= Margin.high` whenever present, `InsufficientData.missing` non-empty, `C*` tags consistent with `missing`. |
| P-04 | Generate the same claim, uncertainty and bounds expressed in different unit pairs: `(W, kW, MW)` and `(J, kWh)`. Values are built as integer multiples so the conversion to SI is exact in `f64`, for example `n` kW against `n * 1000` W. Assert `Decision` equality (verdict, enclosures with bounds and margins). A second, real-valued variant asserts equality when the claim is farther than the guard from every bound. |
| P-05 | For each of the 13 symbols: generate `x` from normal finite `f64` in `[-1e12, 1e12]` and zero. Assert `approx_eq(from_unit(x, s).to_unit(s), x)` with the comparison done in SI (`ABS_TOL` applies for zero). Also check each row of the unit table against the table in 3.4. |
| P-06 | Generate pairs of unit symbols with different dimensions (for example `W`/`J`, `m`/`s`, `kg`/`mol`) as claim against limit kind, and as `u` against `v`. Assert `Err(IncompatibleDimension)` and that no `Ok` is returned. Also `AbsoluteTemperature` against `TemperatureDifference` gives `Err(TemperatureKindMismatch)`. |
| P-10 | Run `decide` twice on the same input and on clones built with constraints inserted in a different order. Assert the two `Decision` values are equal. |
| E-01 | `guard_rel = guard_abs = 0`, `u = 0`, any `k`. Oracle: the original rules (`v > B_max`, `v <= B_min`, otherwise `Conditional` with `C* = { c : B(c) >= v }`; Min-bound mirrored). Assert equality with `decide`. |
| E-02 | Generate `u1 <= u2`, all three guard modes. Assert: if the verdict for `u1` is not `Impossible`, the verdict for `u2` is not `Impossible`; if the verdict for `u1` is not `Compatible`, the verdict for `u2` is not `Compatible`. Run with and without missing parameters. |
| E-03 | Generate a limit and a constraints map with at least one empty allowed set, or a limit with an empty domain. Assert `Err(EmptyConfigurationSet)` and that no verdict is returned. |
| E-04 | Any input where the verdict is `Conditional` (filtered by assertion on the result, with enough generated cases that most runs hit it): `satisfying` is non-empty, for both directions, all guard modes, and with missing parameters. |

### 9.4 Example-based tests (requirements without a property)

These supplement the properties and never replace them.

- R-02.4 (D-18, D-21): non-finite `v`, `u`; `u < 0`; `k <= 0`; `guard_rel` equal to 1, above 1, negative, NaN or infinite gives `InvalidGuard`; `guard_rel = 0` and `guard_rel = 0.999` are accepted; negative or non-finite `guard_abs` gives `InvalidGuard`; `max_configurations = 0` gives `InvalidMaxConfigurations`.
- R-02.5: negative bound.
- R-08.4: constraint outside domain.
- R-08.6 (D-17): a limit whose narrowed `|S|` is above the cap returns `ConfigurationSetTooLarge` quickly, without enumeration. A product that overflows `usize` (many large domains) returns the same error and does not panic. A product above the cap before narrowing but within the cap after narrowing succeeds.
- R-08.7: a plausible interval `[0, 1e300]` is handled in constant time.
- D-15 boundary: `v = B + g(B)` (computed with the oracle formula) is not `Impossible`; its `next_up` is `Impossible` (with `u = 0`). Mirrored for Min-bound. `Compatible` at `hi == B_min` exactly, and not one ulp above.
- D-12: a negative claim gives a margin with `low <= high` equal to the hull of the four ratios; `B_min == 0` gives `None` and no division.
- D-16, D-22 and R-05.5: one missing parameter without domain, plus one with a domain, gives `InsufficientData`, `enclosures == None`, and the list contains both parameters tagged `NoDomain` and `HasDomain`.
- D-03, D-10, D-13, D-19: two or three hand-written scenarios with missing parameters, with `Certain` and `Depends(all missing)` tags. One scenario puts the restrictive end exactly at `lo - g(...)` so that the guard decides between `Certain` and `Depends` (Max-bound), and its mirror (Min-bound).
- R-07.5: the Display text of every `EngineError` and every public doc string avoid the listed words.

## 10. Open questions

None. The former OQ-01 to OQ-05 are decisions D-18 to D-22 in `requirements.md`.
## 11. Amendment D-24 (spec limit-author)
`decide` step 7 uses `guard_abs = max(cfg.guard_abs(), limit.guard_abs())`. `Limit::guard_abs` is an optional trait method with default 0, validated there (`InvalidGuard` if negative or not finite). `DecisionConfig::default()` has `guard_abs = DecisionConfig::DEFAULT_GUARD_ABS = 0`. This supersedes the D-20 row of the traceability table ("`Limit` has no guard method") and the note in section 4 that there is no per-limit guard. Tests: the guard modes of 9.2 gain `AbsFloor` (`rel = 1e-9`, `abs = 1e-12`, the former default); the two slice-5 regression cases keep that explicit floor; `tests/guard_scale.rs` checks a Landauer-sized bound, a zero bound with a declared floor, and a scale property over bounds in `[1e-30, 1e30]` with the fixed seed. Properties P-01 to P-10 and E-01 to E-04 are unchanged.
