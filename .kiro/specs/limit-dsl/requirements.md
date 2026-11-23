# Requirements: limit-dsl

Status: v1, frozen. Base: `decision-engine` v5, `limit-betz`, `limit-carnot`, `limit-cop` (all unchanged). New crate `limina-dsl`. `limina-core`, `limina-api` and `limina-extract` are not modified.

## Summary

A physical limit can be written as a JSON definition instead of Rust code. Its formulas are checked for dimensional consistency when it is loaded, then it runs in the existing decision engine through the core `Limit` trait (`DeclarativeLimit`). No language model, no network, no HTTP.

## Facts

- Definition: `id`, `title`, `direction` (`max` or `min`), `bounded_dimension` (SI exponents, or a unit of the core unit table), `parameters` (name, `dimension` or `kind`, `units`, `required`, optional `plausible` domain with its `source`, optional `test_range`), `assumptions` (id and options; an option may carry its own `formula` and/or `condition`), `formula` (shared; options may override), `validity` (condition strings such as `"Th > Tc"`), `references`, `status` (`draft` by default; `validated` only with a human `validated_by`), `epistemic_status`.
- Expression grammar: decimal numbers, parameter names, exact SI constants `kB` = 1.380649e-23 J/K, `h` = 6.62607015e-34 J s, `c` = 299792458 m/s, `e` = 1.602176634e-19 C, `NA` = 6.02214076e23 1/mol, `pi` and `ln2` (dimensionless), operators `+ - * / ^` (integer exponents), parentheses, functions `sqrt ln exp abs min max`. Nothing else.
- Conditions: one comparison `<`, `<=`, `>`, `>=` between two expressions.

## Functional requirements

- **R-01 Static dimensional analysis** with the core `Dimension`: `+` and `-` need equal dimensions; `ln` and `exp` need dimensionless arguments; `sqrt` needs even exponents; the formula dimension must equal `bounded_dimension`, else `DimensionMismatch` with the computed and the declared dimensions; an unknown name is `UnknownSymbol`; a parse error carries its position. Absolute temperatures and temperature differences follow the core rules: absolute minus absolute is a difference, absolute plus difference is absolute, absolute plus absolute is refused; a number has no unit, so `T + 273.15` is refused (no silent Celsius).
- **R-02 Evaluation** with the core `Interval`, so missing parameters with a plausible domain are enclosed by the engine. Naive interval evaluation of an expression where a variable appears more than once is conservative (wider than the true range); it can only widen the bound range, never create a false IMPOSSIBLE.
- **R-03 Typed check report** for any definition: (a) dimension check; (b) monotonicity per parameter, sampled over the plausible domain or the `test_range` (increasing, decreasing, non-monotone, undetermined); (c) finiteness and non-negativity of the bound on the sampled range; (d) validity conditions on the sampled range. A definition is reported validated only when `status` is `validated` and `validated_by` is set.
- **R-04 Registry** loading `data/limits/*.json` and exposing each definition as a `Limit`.
- **R-05 Definitions** `data/limits/betz.json`, `carnot.json`, `cop.json`: same assumptions, options, formulas and validity rules as the Rust limits, status `draft`, reference TODO unchanged.

## Acceptance criteria

1. Differential properties (proptest, fixed seed, 256 cases, 1024 with `CI` set): for betz, carnot and cop, the DSL limit and the Rust limit return the same bound (relative tolerance 1e-12) and the same typed error for the same valid or invalid inputs, and `decide` returns the same verdict and the same C* on generated claims, constraints and parameters. A failure prints a readable counterexample.
2. Dimension checker: Carnot `1 - Tc/Th` dimensionless passes; `Tc - Th` declared dimensionless fails with `DimensionMismatch`; `kB*T` has the dimension of energy; `ln(T)` and `sqrt(T)` fail.
3. Parser errors carry positions; unknown names are `UnknownSymbol`.
4. Interval conservativeness is tested.
5. The monotonicity report on carnot: increasing in `Th`, decreasing in `Tc`.
6. A `draft` definition is never reported as validated.

## Out of scope

Any change to the existing crates or specs; HTTP access to the DSL; editing definitions from the web app.
