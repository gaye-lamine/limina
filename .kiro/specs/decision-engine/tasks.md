# Tasks: decision-engine

Based on `requirements.md` v5 (frozen) and `design.md` v5 (frozen). Crate: `limina-core`.

Rules for every task:

- One file or one function group, plus its tests.
- After the task, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass.
- No `unwrap()` in library code. Public items documented.
- A task adds only what it lists. Later tasks extend earlier files.

## Slice 0: dependencies

- [x] **T01 Remove `uom`.** Remove `uom` from `crates/limina-core/Cargo.toml`; let `Cargo.lock` update. No code change.
  Covers: D-14.

## Slice 1: Dimension, Quantity, unit table

- [x] **T02 `dimension.rs` and initial `error.rs`.** `Dimension` (7 `i8` exponents, `DIMENSIONLESS`, `new`, `exponents`). `EngineError` with only the variants needed by slice 1: `IncompatibleDimension`, `UnknownUnit`, `NonFiniteValue`, `TemperatureKindRequired`, `TemperatureKindMismatch`, `NegativeAbsoluteTemperature`. Unit tests: equality, ordering, constructor.
  Covers: R-01.1, R-06.4.
- [x] **T03 `units.rs`.** `UnitEntry`, `lookup_unit`, the table of exactly 13 symbols (design 3.4). Exact symbol match; anything else gives `UnknownUnit`. Unit tests: one per row (dimension and factor), unknown symbols (`kw`, `W/m^2`, `°C`).
  Covers: R-01.1, design 3.4.
- [x] **T04 `Quantity` in `quantity.rs`.** `from_unit`, `from_si`, `to_unit`, `si_value`, `dimension`. `K` gives `TemperatureKindRequired`; non-finite gives `NonFiniteValue`; `to_unit` to another dimension gives `IncompatibleDimension`. Unit tests for each error and for `kW` to `W`, `kWh` to `J`.
  Covers: R-01.1, R-01.2.
- [x] **T05 Temperature and measure types in `quantity.rs`.** `AbsoluteTemperature`, `TemperatureDifference` and their allowed operations only, `Measure`, `MeasureKind`, `kind`, `si_value`, `uncertainty_kind`. Unit tests: negative kelvin rejected, `abs - abs = diff`, `abs + diff` checked, absolute against difference gives `TemperatureKindMismatch`.
  Covers: R-01.2, R-01.4, R-01.5.

## Slice 2: Interval

- [x] **T06 `interval.rs` core.** `Interval` with `new`, `point`, `lo`, `hi`, `hull`, `add`, `sub`. Add `InvertedInterval` to `EngineError`. Unit tests: invariants, non-finite rejected, add and sub on known values.
  Covers: R-02.6.
- [x] **T07 `interval.rs` mul and div.** `mul`, `div`. Add `DivisionByIntervalContainingZero`. Unit tests: sign cases of `mul`, `div` by an interval containing 0 rejected.
  Covers: R-02.6.

## Slice 3: DecisionConfig and error types

- [x] **T08 `tolerance.rs`.** `REL_TOL`, `ABS_TOL`, `approx_eq`, `guard`. Unit tests: `approx_eq` at 0 and at large values; `guard` takes the max of the two terms.
  Covers: D-06, D-15 (formula of `g`).
- [x] **T09 `settings.rs`.** `DecisionConfig` with private fields, `new`, getters, `Default`, `DEFAULT_K`, `DEFAULT_MAX_CONFIGURATIONS`. Add `InvalidCoverageFactor`, `InvalidGuard`, `InvalidMaxConfigurations`. Unit tests: `k <= 0`, NaN, infinite; `guard_rel` equal to 1, above 1, negative, NaN, infinite rejected; `guard_rel = 0` and `0.999` accepted; negative or non-finite `guard_abs` rejected; `max_configurations = 0` rejected; defaults.
  Covers: R-02.3, R-02.4 (config part), D-18, D-20, D-21.
- [x] **T10 Complete `error.rs`.** Add `NegativeUncertainty`, `InvalidBound`, `UnknownAssumption`, `ConstraintOutsideDomain`, `UnknownParameter`, `EmptyConfigurationSet`, `ConfigurationSetTooLarge`. Test: the `Display` text of every variant avoids the words fraud, lie, scam, hoax.
  Covers: R-06.4, R-07.5.

## Slice 4: Limit trait and ConfigurationSet

- [x] **T11 `limit.rs`.** `AssumptionId`, `OptionId`, `ParameterId`, `Direction`, `AssumptionOption`, `Assumption`, `DomainRange`, `PlausibleDomain`, `ParameterSpec`, `ParameterEnclosure`, `Configuration`, trait `Limit`. Add `tests/common/mod.rs` with `TestLimit` (table of bound values, no physics). Unit tests: id ordering, `ParameterEnclosure::get`, `TestLimit::bound` on a point enclosure.
  Covers: R-08.1, R-08.7, D-04, D-20 (no guard on `Limit`).
- [x] **T12 `config.rs`.** `ConstraintsMap`, `ConfigurationSet::build` with the check order of design 5.1 and the size cap computed with checked multiplication before enumeration. Unit tests: unconstrained keeps the full domain; unknown assumption; constraint outside domain; empty allowed set and empty domain give `EmptyConfigurationSet`; above the cap gives `ConfigurationSetTooLarge` without enumeration; `usize` overflow gives the same error without panic; above the cap before narrowing but within it after narrowing succeeds; lexicographic order.
  Covers: R-08.1 to R-08.6, D-07, D-08, D-17.
- [x] **T13 `bound_range` in `engine.rs`.** `EnclosureKind`, `BoundRange` (in `verdict.rs`), `bound_range` with `InvalidBound` for negative or non-finite ends. Unit tests on `TestLimit` with point enclosures.
  Covers: R-02.5, R-02.6.

## Slice 5: verdict rules without missing parameters

- [x] **T14 `verdict.rs` types.** `MissingStatus`, `MissingParameter`, `Support`, `SatisfyingConfig`, `Verdict`, `Margin`, `EnclosureResult`, `Enclosures`, `Decision`; and `Claim`, `DecisionInput`. Types only, with doc comments. Unit test: construction and equality.
  Covers: R-07.1 to R-07.3 (types).
- [x] **T15 Input validation (step 1).** Private function in `engine.rs`: claim kind against `bounded_kind`, kind of `u`, finite `v` and `u`, `u >= 0`, computation of `lo` and `hi` with `u = 0` when absent. Unit tests for each error and for `lo`, `hi`.
  Covers: R-01.3, R-01.4, R-02.1, R-02.2, R-02.4 (claim part).
- [x] **T16 Rules (step 7).** Private `rules_max` and `rules_min` over one enclosure: guarded `Impossible` as an explicit quantifier over `S`, exact `Compatible`, otherwise `Conditional`; `debug_assert!` against the `B_max` / `B_min` form. Unit tests at the boundaries: `v = B + g(B)` not `Impossible`, its `next_up` is `Impossible` (`u = 0`); `Compatible` at `hi == B_min` and not one ulp above; mirrored for Min-bound; guard = 0 case.
  Covers: R-03.1, R-03.2, R-04.1, R-04.2, R-09, D-01, D-02, D-15, D-18.
- [x] **T17 C\* (step 9, no missing parameters).** Private function: guarded membership on each `c`, every member `Certain`. Unit tests: membership at the guard boundary for both directions; the configuration at the extreme is always kept.
  Covers: R-03.3, R-04.3, D-15, E-04 (by example).
- [x] **T18 Margin (step 10).** Private function: hull of the four ratios when `B_min > 0`, `None` when `B_min == 0` with no division. Unit tests: positive claim equals `[lo / B_max, hi / B_min]`; negative claim gives `low <= high`; zero bound gives `None`.
  Covers: D-09, D-12.
- [x] **T19 `decide` end to end, no missing parameters.** Public `decide` wiring steps 1, 2, 3 (supplied parameters only, kind check, `UnknownParameter`), 5, 6 (both enclosures equal), 7, 9, 10. Until slice 7, a missing required parameter returns `InsufficientData` with all missing parameters tagged and `enclosures: None` (conservative, never `Impossible`). Unit tests: one scenario per verdict for each direction; `UnknownParameter`; `EmptyConfigurationSet` through `decide`.
  Covers: R-03, R-04, R-06.1 to R-06.3, R-07.1, R-07.2.

## Slice 6: properties

- [x] **T20 Test infrastructure (`tests/common/`).** Fixed-seed `runner()` (constant 32-byte seed, 256 cases, 1024 in CI via a constant), `Scenario` with a hand-written readable `Debug`, shared generators (bounds as integers `0..=1000`, 1 to 3 assumptions with 1 to 5 options, claim as `bound + offset`, `u` and `k` as integers, four guard modes), independent oracle with its own guard formula. Commit `proptest-regressions/`; create `tests/regressions.rs` (empty). Module doc notes that P-07 to P-09 are out of scope. One smoke property to prove the runner works. Check the proptest 1.12.0 API for the seeded runner.
  Covers: testing strategy 9.1, 9.2, D-05.
- [x] **T21 P-05 and P-06.** `tests/units.rs`.
  Covers: P-05, P-06, D-06.
- [x] **T22 P-04.** `tests/units_equivalence.rs`: integer-multiple variant and real-valued variant away from the guard.
  Covers: P-04.
- [x] **T23 P-01 and P-10.** `tests/sets.rs`.
  Covers: P-01, P-10, R-06.1.
- [x] **T24 P-02, P-03 and E-01.** `tests/rules.rs`, no missing parameters, both directions, four guard modes.
  Covers: P-02, P-03 (a), P-03 (b), E-01, R-09.
- [x] **T25 E-02, E-03 and E-04.** `tests/engine_props.rs`, no missing parameters.
  Covers: E-02, E-03, E-04.

## Slice 7: missing parameters, enclosures, Certain / Depends

- [x] **T26 Parameter resolution and NoDomain (steps 3 and 4).** Missing parameters tagged `HasDomain` (hull of the plausible domain, never enumerated) or `NoDomain`; any `NoDomain` gives `InsufficientData` with all missing parameters and `enclosures: None`, without calling `Limit::bound`. Replaces the temporary behaviour of T19. Unit tests: one `NoDomain` plus one `HasDomain` parameter; a plausible interval `[0, 1e300]` handled in constant time.
  Covers: R-05.5, R-05.9, R-07.3, R-07.4, R-08.7, D-04, D-16, D-22.
- [x] **T27 Two enclosures (steps 5 and 6).** Permissive and restrictive selection per direction, `Enclosures` with bounds and margin per enclosure. Unit tests: Max-bound and Min-bound selection on a `TestLimit` with one parameter.
  Covers: R-05.1, R-05.2, R-07.1, D-11.
- [x] **T28 Combine (step 8).** Same verdict under both enclosures returned as is; different verdicts give `InsufficientData` with all missing parameters. Unit tests: agree, disagree, and `Impossible` only under both.
  Covers: R-05.3, R-05.4, R-05.6, D-03.
- [x] **T29 Certain / Depends (step 9 with missing parameters).** Membership on the permissive end, `Certain` on the restrictive end with the guarded comparison, otherwise `Depends(all missing parameters)`. Unit tests: two or three scenarios; one with the restrictive end exactly at the guard boundary, both directions.
  Covers: R-05.7, R-05.8, D-10, D-13, D-19.
- [x] **T30 Properties with missing parameters.** Extend the generators with parameters and plausible domains; run P-02 (permissive variant), P-03 (b), E-02 and E-04 with missing parameters. Move any shrunk failure found so far into `tests/regressions.rs`.
  Covers: P-02, P-03 (b), E-02, E-04, R-05, R-09.

## Follow-up

- [x] **T31** test that the public doc strings and EngineError messages avoid the words fraud, lie, scam, hoax, covering R-07.5 as design.md 9.4 requires; match whole words only.
  Covers: R-07.5.

- Amended by api-limit-metadata, unit lookup: `units_for_dimension` added to `units.rs` (additive, no existing code changed).
- Amended by limit-author, D-24: default `guard_abs` = 0 and optional `Limit::guard_abs` floor (see requirements.md D-24, design.md section 11).
- Amended by si-units-constants, D-25: prefixed and compound SI units (units_parse.rs) and CODATA 2022 constants (constants.rs).
