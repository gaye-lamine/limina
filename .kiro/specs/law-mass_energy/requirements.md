# Requirements: law-mass_energy
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/mass_energy.json`. Added after the 26/11 review of a multi-domain image.
## Facts
- Name: Mass-energy equivalence. Kind: max bound. Bounded quantity dimension: `J`.
- Equation: `dm*c^2`
- Classification: `fundamental_law`. Epistemic status: `theorem`.
- Validity domain: The energy released by converting a mass dm into energy is at most dm c^2; complete conversion (matter-antimatter annihilation) reaches it, nuclear reactions convert only a fraction of their mass.
- Validity conditions: none in the DSL
- Variables:
- `dm`: Mass converted into energy (kg, g, mg)
## Limits of application
- dm is the mass converted (the mass defect), not the mass of the fuel when only part of it is converted
- An upper bound: reactors release a small fraction of it
## References
- Einstein, A., Ist die Trägheit eines Körpers von seinem Energieinhalt abhängig?, Annalen der Physik 323(13), 639-641 (1905), doi:10.1002/andp.19053231314
- CODATA 2022 recommended value of the speed of light in vacuum, https://physics.nist.gov/cgi-bin/cuu/Value?c
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property in `crates/limina-dsl/tests/review_laws.rs`, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound in every configuration allowed by the text is IMPOSSIBLE with the wording "inconsistent with limit mass_energy under assumptions ..."; when an assumption is not stated and one option allows the claim, the verdict is CONDITIONAL.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out, not by the DSL.
- **R-06** The built-in matcher (`crates/limina-extract/src/matcher.rs`) chooses the law by keyword and dimension; the model's limit id is a hint only.
