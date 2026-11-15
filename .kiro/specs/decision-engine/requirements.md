# Requirements: decision-engine

Crate: `limina-core`. Status: v5, frozen.

## Introduction

Given a claimed value `v` with an optional uncertainty `u`, and a physical limit with a bound function `B(c, p)` over a set `S` of configurations compatible with the text, the decision engine computes a verdict. The verdict is computed only by this deterministic engine, never by an LLM.

## Glossary

- **Claim**: a quantity `v` extracted from a text, with an optional uncertainty `u` (same dimension as `v`).
- **Decision config `DecisionConfig`**: the engine settings `{ k, guard_rel, guard_abs, max_configurations }`, with the defaults of R-02.3.
- **Coverage factor `k`**: multiplier applied to `u`. Default 2. Constraint: `k > 0`.
- **Guard `g(x)`**: `g(x) = max(guard_rel * |x|, guard_abs)`. Defaults: `guard_rel = REL_TOL = 1e-9`, `guard_abs = ABS_TOL = 1e-12` (SI base units). Constraints: `0 <= guard_rel < 1` and `guard_abs >= 0` (D-18). Global, not per limit (D-20). Used only by the `IMPOSSIBLE` rules and by `C*` (see D-15).
- **Claim interval `[lo, hi]`**: `lo = v - k*u`, `hi = v + k*u`, with `u >= 0`. When `u` is absent, `u = 0` and `lo = hi = v`.
- **Assumption**: a named parameter of a limit, with a finite domain of values declared by the limit.
- **Configuration `c`**: one assignment of a value to every assumption of a limit.
- **Constraints map**: a map from assumption id to an allowed subset of that assumption's domain, derived upstream from the text.
- **Set `S`**: the product of the assumption domains narrowed by the constraints map.
- **Parameter `p`**: any other input of `B` (for example temperatures), taken from the claim or from the text.
- **Plausible domain**: for a parameter, an interval or a finite set declared by the limit, with its source.
- **Bound `B(c, p)`**: the value of the limit for configuration `c`.
- **`B_max`, `B_min`**: the maximum and minimum of `B(c, p)` over `c` in `S`.
- **Direction**: `Max` (the claim must not exceed the bound) or `Min` (the claim must be at least the bound).
- **Enclosures**: the pair `{ permissive, restrictive }`, each with its own `B_min`, `B_max` and margin (see D-11).
- **`C*`**: the set of configurations that satisfy the claim, returned with a `CONDITIONAL` verdict. Each member is tagged `Certain` or `Depends(list of missing parameters)` (see R-05 and D-10, D-13, D-19).
- **Verdict**: one of `IMPOSSIBLE`, `CONDITIONAL`, `COMPATIBLE`, `INSUFFICIENT_DATA`.
- **Margin**: an interval, defined in D-12.

## Requirements

### R-01 Typed quantities and dimensional checking

1. WHEN a claim, an uncertainty, a parameter or a bound is provided, THE engine SHALL represent it as a typed physical quantity carrying its dimension.
2. WHEN two quantities of incompatible dimensions are compared or combined, THE engine SHALL return a typed error and SHALL NOT convert one into the other.
3. WHEN the claim has a dimension different from the bounded quantity of the limit, THE engine SHALL return a typed error.
4. WHEN `u` is provided, THE engine SHALL require that `u` has the same dimension as `v`, otherwise it SHALL return a typed error.
5. THE engine SHALL use absolute temperatures (kelvin) internally.

### R-02 Interval arithmetic for uncertainty

1. THE engine SHALL treat the claim as the interval `[lo, hi]` with `lo = v - k*u` and `hi = v + k*u`.
2. WHEN `u` is absent, THE engine SHALL use `u = 0`.
3. THE engine SHALL take its settings from a `DecisionConfig { k, guard_rel, guard_abs, max_configurations }`. The defaults are `k = 2`, `guard_rel = REL_TOL`, `guard_abs = ABS_TOL`, `max_configurations = 100000`. The caller MAY override each field. The guard is global; there is no per-limit guard (D-20).
4. IF `v` or `u` is not finite, OR `u < 0`, OR `k` is not finite, OR `k <= 0`, OR `guard_rel` is not finite or is outside `0 <= guard_rel < 1` (D-18), OR `guard_abs` is not finite or is negative, OR `max_configurations` is 0 (D-21), THEN THE engine SHALL return a typed error. An invalid guard gives `InvalidGuard`.
5. IF any bound `B(c, p)` is negative or not finite, THEN THE engine SHALL return a typed error.
6. THE engine SHALL compute `B_max` and `B_min` over `S` using interval arithmetic where `B` depends on uncertain inputs.

### R-03 Verdict for a Max-bound limit

Rules apply to the claim interval `[lo, hi]` (see D-01, D-15).

1. WHEN `lo > B_max + g(B_max)`, THE engine SHALL return `IMPOSSIBLE`.
2. WHEN `hi <= B_min` (exact comparison, no guard), THE engine SHALL return `COMPATIBLE`.
3. OTHERWISE, THE engine SHALL return `CONDITIONAL` together with `C* = { c in S : B(c) + g(B(c)) >= lo }`.
4. WHEN a required parameter is missing, THE engine SHALL apply R-05 instead of the rules above.

### R-04 Verdict for a Min-bound limit

Mirror of R-03 (see D-01, D-02, D-15).

1. WHEN `hi < B_min - g(B_min)`, THE engine SHALL return `IMPOSSIBLE`.
2. WHEN `lo >= B_max` (exact comparison, no guard), THE engine SHALL return `COMPATIBLE`.
3. OTHERWISE, THE engine SHALL return `CONDITIONAL` together with `C* = { c in S : B(c) - g(B(c)) <= hi }`.
4. WHEN a required parameter is missing, THE engine SHALL apply R-05 instead of the rules above.

### R-05 Missing parameters (see D-03, D-04, D-11, D-13, D-16, D-19, D-22)

1. WHEN a required parameter is missing AND the limit declares a plausible domain for every missing required parameter, THE engine SHALL evaluate the verdict, with the rules of R-03 or R-04, under the most permissive and under the most restrictive enclosure of the missing parameters.
2. For a Max-bound limit, permissive means the largest bound and restrictive means the smallest. For a Min-bound limit this is reversed.
3. WHEN both verdicts are identical, THE engine SHALL return that verdict.
4. WHEN the two verdicts differ, THE engine SHALL return `INSUFFICIENT_DATA` together with the list of ALL missing parameters, each tagged `NoDomain` or `HasDomain`.
5. WHEN any required missing parameter has no declared plausible domain, THE engine SHALL return `INSUFFICIENT_DATA` directly and SHALL NOT compute any enclosure (see D-22). The list contains ALL missing parameters, each tagged `NoDomain` or `HasDomain`.
6. THE engine SHALL return `IMPOSSIBLE` only if the verdict is `IMPOSSIBLE` under the permissive enclosure.
7. WHEN R-05 applies AND the verdict is `CONDITIONAL` under both enclosures, THE engine SHALL compute `C*` under the permissive enclosure (see D-10, D-15):
   - Max-bound: `c` is in `C*` if and only if `U + g(U) >= lo`, where `U` is the upper end of `B(c)` over the plausible ranges of the missing parameters.
   - Min-bound: `c` is in `C*` if and only if `W - g(W) <= hi`, where `W` is the lower end of `B(c)` over the plausible ranges of the missing parameters.
8. THE engine SHALL tag each `c` in `C*` using the same guarded comparison applied to the restrictive end of the plausible range (see D-19):
   - Max-bound: `Certain` if and only if `L + g(L) >= lo`, where `L` is the lower end of `B(c)` over the plausible ranges of the missing parameters.
   - Min-bound: `Certain` if and only if `V - g(V) <= hi`, where `V` is the upper end of `B(c)` over the plausible ranges of the missing parameters.
   - Otherwise `Depends(list of missing parameters)`. The list contains all missing parameters of the limit (see D-13).
9. THE engine SHALL NOT invent a missing quantity, SHALL NOT guess a plausible domain, and SHALL NOT take any value from an LLM. A missing quantity SHALL be reported as missing.

### R-06 Determinism, purity and isolation

1. THE engine SHALL be deterministic: identical inputs SHALL produce identical outputs.
2. THE engine SHALL NOT depend on an LLM or on any provider-specific code.
3. THE `limina-core` crate SHALL NOT perform I/O (no network, no file system, no clock, no randomness, no environment access).
4. Library code SHALL NOT use `unwrap()`. Errors SHALL be typed with `thiserror`.

### R-07 Output

1. THE engine SHALL return the verdict, the inputs used (`v`, `u`, `k`, `guard_rel`, `guard_abs`), and, WHEN enclosures were computed, `Enclosures { permissive, restrictive }`, each with its `B_min`, `B_max` and margin (see D-11, D-12). WHEN no parameter is missing, the two enclosures are equal.
2. WHEN the verdict is `CONDITIONAL`, THE output SHALL include `C*` with the `Certain` or `Depends(...)` tag of each member.
3. WHEN the verdict is `INSUFFICIENT_DATA`, THE output SHALL include the list of missing parameters with their `NoDomain` or `HasDomain` tag.
4. WHEN the verdict is `INSUFFICIENT_DATA` because a missing parameter has no declared domain (R-05.5), THE output SHALL contain no enclosures.
5. THE engine SHALL NOT use the words fraud, lie, scam or hoax in any output text.

### R-08 Configuration set (see D-07, D-08, D-17)

1. THE engine SHALL receive the limit (with its declared assumption domains) and a constraints map.
2. THE engine SHALL compute `S` as the product of the assumption domains narrowed by the constraints map.
3. WHEN an assumption has no entry in the constraints map, THE engine SHALL keep its full declared domain.
4. IF a constraint contains a value outside the declared domain of its assumption, THEN THE engine SHALL return a typed error.
5. IF `S` is empty, THEN THE engine SHALL return the typed error `EmptyConfigurationSet` and SHALL NOT return a verdict.
6. IF `|S|` exceeds `max_configurations`, THEN THE engine SHALL return the typed error `ConfigurationSetTooLarge` and SHALL NOT return a verdict. The size SHALL be checked before `S` is enumerated.
7. THE engine SHALL NOT enumerate a plausible range. A plausible range is represented by its interval endpoints only.

### R-09 Core safety invariant

1. THE engine SHALL return `IMPOSSIBLE` only if the claim interval exceeds the bound by more than the guard, for every configuration in `S`, under the permissive enclosure of missing parameters.
2. Max-bound: this means `lo > B(c) + g(B(c))` for every `c` in `S`. Min-bound: this means `hi < B(c) - g(B(c))` for every `c` in `S`. `B(c)` is taken under the permissive enclosure.
3. THE engine SHALL NOT return `IMPOSSIBLE` when the claim is within the guard of any bound (Max-bound: `lo <= B(c) + g(B(c))` for some `c`; Min-bound: `hi >= B(c) - g(B(c))` for some `c`).
4. This invariant is independent of D-03. D-03 covers missing parameters only. R-09 applies in all cases, including when no parameter is missing.

## Property-based requirements

Each property SHALL have a `proptest` test. CI SHALL use fixed seeds. Failing cases SHALL be kept as regression tests. Shrunk counterexamples SHALL print in a human-readable format.

- **P-01** widening S never lowers B_max nor raises B_min.
- **P-02** never classify a value at or below B(c), for any c in S, as IMPOSSIBLE. IMPOSSIBLE is also never returned when the claim is within the guard of any bound (Max-bound: `lo <= B(c) + g(B(c))` for some `c`; Min-bound: `hi >= B(c) - g(B(c))` for some `c`). (Verifies R-09.)
- **P-03**
  - (a) with no missing parameters, exactly one of the three rules of R-03 (Max-bound) or R-04 (Min-bound) applies, for all `v` and `u >= 0`, with `S` non-empty, with exact boundaries. The boundaries are those of the guarded rules (see D-01, D-15).
  - (b) in all cases, exactly one verdict is returned.
- **P-04** equivalent units (W/kW/MW, J/kWh) give the same verdict and margin.
- **P-05** unit conversion round trip returns the original value within tolerance (see D-06).
- **P-06** an incompatible dimension returns a typed error.
- **P-10** the same input gives an identical output.

Engine-level properties (named E-xx):

- **E-01** with `guard_rel = guard_abs = 0` and `u = 0` the rules equal the original rules (Max-bound: IMPOSSIBLE if v > B_max; COMPATIBLE if v <= B_min; CONDITIONAL if B_min < v <= B_max with C* = { c : B(c) >= v }; Min-bound mirrored).
- **E-02** increasing u never turns a non-IMPOSSIBLE verdict into IMPOSSIBLE, and never turns a non-COMPATIBLE verdict into COMPATIBLE.
- **E-03** an empty S always returns the typed error.
- **E-04** C* is non-empty whenever the verdict is CONDITIONAL.

## Decisions

Recorded from author review of the former open questions. Notation: `lo = v - k*u`, `hi = v + k*u`, `u >= 0`, `k > 0`.

- **D-01 (was Q-01) Rules on the claim interval.** The verdict rules are on `[lo, hi]` as written in R-03 and R-04. With `u = 0` and no guard (`guard_rel = guard_abs = 0`) they reduce exactly to the original rules. When `S` is non-empty, the three outcomes are exhaustive and mutually exclusive, so P-03 holds for every `v` and `u >= 0`. `C*` is never empty when the verdict is `CONDITIONAL`. The `IMPOSSIBLE` rule and `C*` use the guard of D-15, which deviates from the strict inequality first written in D-01.
- **D-02 (was Q-02) Min-bound.** Min-bound means the quantity must be at least `B(c)`. The rules in R-04 are the intended inversion and replace the earlier reading.
- **D-03 (was Q-03) Priority of INSUFFICIENT_DATA.** Missing parameters are not a verdict by themselves. The verdict is evaluated under the permissive and the restrictive enclosure (R-05). Identical verdicts are returned as is. Different verdicts give `INSUFFICIENT_DATA` with the list of missing parameters. `IMPOSSIBLE` therefore requires `IMPOSSIBLE` under the permissive enclosure.
- **D-04 (was Q-04) Plausible values.** Each limit declares, per parameter, a plausible domain (interval or finite set) with its source, written in that limit's spec. The engine never guesses and never takes values from an LLM. A required parameter that is missing and has no declared domain gives `INSUFFICIENT_DATA` directly.
- **D-05 (was Q-05) Property numbering.** P-07, P-08 and P-09 are intentionally absent. They belong to limit-specific specs and to the dataset. They are not defined in this spec.
- **D-06 (was Q-06) Tolerance for P-05.** Relative tolerance `REL_TOL = 1e-9`, plus an absolute tolerance `ABS_TOL = 1e-12` in SI base units for zero values. Both SHALL be defined as named constants.
- **D-07 (was Q-07) Empty S.** A typed error `EmptyConfigurationSet`, never a verdict.
- **D-08 (was Q-08) Origin of S.** The engine receives the limit and a constraints map (assumption id to allowed subset, derived upstream from the text) and computes `S` as in R-08. A constraint outside a declared domain is a typed error. An unconstrained assumption keeps its full domain.
- **D-09 (was Q-09) Margin.** The margin is the interval `[lo / B_max, hi / B_min]`, valid when all `B(c) > 0`. If any `B(c) = 0`, the margin is `None`, no division is performed, and the verdict is unaffected. Reading depends on direction: Max-bound, a value `<= 1` is within the bound; Min-bound, a value `>= 1` is within the bound. Superseded in form by D-12; the two agree for `lo >= 0`.
- **D-10 (was O-01) Content of C* with missing parameters.** When R-05 applies and the verdict is `CONDITIONAL` under both enclosures, `C*` is computed under the permissive enclosure (R-05 items 7 and 8). Each member is tagged `Certain` (the condition holds over the whole plausible range) or `Depends(list of missing parameters)`. `C*` is therefore never empty when the verdict is `CONDITIONAL`, so E-04 still holds.
- **D-11 (was OQ-01) Enclosures output.** When parameters are missing, the output returns both enclosures: `Enclosures { permissive, restrictive }`, each with its bounds and margin. If any missing parameter has no declared plausible domain, no enclosure is computed and the verdict is `INSUFFICIENT_DATA`.
- **D-12 (was OQ-02) Margin with negative claims.** The margin is the hull `[min, max]` of the four ratios `lo/B_max`, `lo/B_min`, `hi/B_max`, `hi/B_min`, valid when all `B(c) > 0`, otherwise `None`. For `lo >= 0` this equals the interval of D-09. This removes inverted intervals for negative claims.
- **D-13 (was OQ-03) Depends list.** `Depends` lists all missing parameters of the limit. This is a known simplification: it may list parameters that `B(c)` does not actually depend on. Minimal dependency sets are not computed.
- **D-14 (was OQ-04) No `uom`.** `uom` is removed from the design and from the dependency plan. The first task of `tasks.md` will be to remove `uom` from `crates/limina-core/Cargo.toml`. `tech.md` is edited by the author by hand, not by this spec.
- **D-15 (was OQ-05) Guard on IMPOSSIBLE.** The engine takes `DecisionConfig { k, guard_rel, guard_abs }` (plus `max_configurations`, D-17) with defaults `guard_rel = REL_TOL` and `guard_abs = ABS_TOL`. With `g(x) = max(guard_rel*|x|, guard_abs)`:
  - Max-bound: `IMPOSSIBLE` iff `lo > B_max + g(B_max)`. Min-bound: `IMPOSSIBLE` iff `hi < B_min - g(B_min)`.
  - The `COMPATIBLE` rules stay exact.
  - `C*` uses the same guarded comparison: Max-bound `{ c : B(c) + g(B(c)) >= lo }`; Min-bound `{ c : B(c) - g(B(c)) <= hi }`. The configuration that attains `B_max` (Max-bound) or `B_min` (Min-bound) always belongs to `C*` when the verdict is not `IMPOSSIBLE`, so `C*` stays non-empty (E-04).
  - E-01 holds with `guard_rel = guard_abs = 0`. P-02 and R-09 say that `IMPOSSIBLE` is never returned when the claim is within the guard of any bound.
  - **Deviation from D-01.** D-01 wrote `IMPOSSIBLE` as a strict, exact inequality. The guard moves the `IMPOSSIBLE` boundary outward by `g`. The reason: the engine has no directed rounding, so a computed endpoint can differ from the exact value by a few ulps, and a false `IMPOSSIBLE` is the worst error (`product.md`). The cost: a claim that exceeds a bound by less than the guard is `CONDITIONAL` instead of `IMPOSSIBLE`.
- **D-16 (was OQ-06) Missing list content.** If any required missing parameter has no declared domain, the verdict is `INSUFFICIENT_DATA` directly. The `missing` list contains ALL missing parameters, each tagged `NoDomain` or `HasDomain`.
- **D-17 (was OQ-07) Cap on |S|.** A hard cap on `|S|`, default 100000, configurable in `DecisionConfig` (`max_configurations`). Exceeding it returns the typed error `ConfigurationSetTooLarge`. Plausible ranges are interval endpoints and are never enumerated.
- **D-18 (was OQ-01 of design v2) Range of `guard_rel`.** `guard_rel` SHALL satisfy `0 <= guard_rel < 1`. Otherwise the engine returns the typed error `InvalidGuard`. This guarantees that `x - g(x)` is non-decreasing in `x >= 0`, so the quantifier form of R-09 and the `B_max` / `B_min` form of D-15 always agree. The validation is part of R-02.4 and is tested.
- **D-19 (was OQ-02 of design v2) `Certain` under the guard.** `Certain` uses the same guarded comparison, applied to the restrictive end of the plausible range. Max-bound: `Certain` iff (lower end of `B(c)` over the missing parameters) + g(lower end) `>= lo`. Min-bound: `Certain` iff (upper end of `B(c)`) - g(upper end) `<= hi`. Otherwise `Depends(all missing parameters)`.
- **D-20 (was OQ-03 of design v2) Guard scope.** The guard stays global in `DecisionConfig`. There is no per-limit guard in this spec.
- **D-21 (was OQ-04 of design v2) Config validations confirmed.** `max_configurations = 0` and a negative or non-finite guard are typed errors, as in R-02.4.
- **D-22 (was OQ-05 of design v2) NoDomain case.** When any required missing parameter has no declared domain, the `missing` list contains all missing parameters tagged `NoDomain` or `HasDomain`, and no enclosures are computed.
