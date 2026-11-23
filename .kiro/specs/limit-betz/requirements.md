# Requirements: limit-betz
Status: v1, frozen. Base: `decision-engine` v5 (unchanged). Crate: `limina-core`.

## Summary
The Betz limit bounds the power coefficient `Cp` of a wind turbine. It is the template for the other limits: one file, one spec, golden tests, example tests, one property test.

## Facts
- Bounded quantity: power coefficient `Cp`, dimensionless. Direction: Max bound. Bound: 16/27 (about 0.593).
- Epistemic status: model bound (actuator disk, incompressible, unconfined flow).
- One assumption, `reference_area`, with three options:
  - `unshrouded_rotor`: bound 16/27.
  - `shrouded_total_capture_area`: bound 16/27.
  - `shrouded_rotor_area_only`: outside the validity domain. The limit returns the typed error `NotApplicable` and computes no bound.
- No numeric parameters, so no missing-parameter handling.
- Reference: TODO, to be supplied by the author. No reference is invented here.

## Functional requirements
- **R-01** The limit declares id `betz`, direction Max, bounded kind dimensionless, the assumption `reference_area` with the three options above, and no parameters.
- **R-02** For `unshrouded_rotor` and `shrouded_total_capture_area`, `bound` returns the point interval 16/27.
- **R-03** For `shrouded_rotor_area_only`, `bound` returns `EngineError::NotApplicable` and computes no bound.
- **R-04** An unconstrained claim keeps all three options in `S`, so `decide` returns `NotApplicable`. This is intended: refusing to decide is never a false IMPOSSIBLE. Callers must constrain `reference_area`.
- **R-05** A later extraction pipeline will map `NotApplicable` to INSUFFICIENT_DATA ("reference area not stated"). That mapping is not part of this spec.
- **R-06** The public documentation gives the formula, the assumptions and the TODO reference. It states that shrouded-turbine claims referenced to the rotor area alone can exceed 16/27, which is why that option is out of domain.

## Acceptance criteria
1. Golden: `bound` equals 16/27 (within `approx_eq`) for `unshrouded_rotor` and for `shrouded_total_capture_area`. No other value is used.
2. Claim 0.30 constrained to `unshrouded_rotor` gives COMPATIBLE.
3. Claim 0.70 constrained to `unshrouded_rotor` gives IMPOSSIBLE.
4. Claim 0.70 constrained to `shrouded_total_capture_area` gives IMPOSSIBLE.
5. Any claim constrained to `shrouded_rotor_area_only` gives `NotApplicable`.
6. An unconstrained claim gives `NotApplicable`.
7. Property: for any claim at or below 16/27 and any non-empty constraint over the two valid options, the engine never returns IMPOSSIBLE.
8. The public documentation of R-06 is present, with the TODO reference clearly marked.

## Out of scope
Numeric parameters, the extraction pipeline and its mapping of `NotApplicable`, any change to `engine.rs` or to the decision-engine spec, the choice of a reference.
