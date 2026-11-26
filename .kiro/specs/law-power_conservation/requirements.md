# Requirements: law-power_conservation
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/power_conservation.json`. Added after the 26/11 review of a multi-domain image.
## Facts
- Name: Conservation of energy (power balance). Kind: max bound. Bounded quantity dimension: `W`.
- Equation: `P_in`
- Option `energy_sources = input_only`: `P_in`
- Option `energy_sources = other_sources`: `1e30*P_in`
- Classification: `fundamental_law`. Epistemic status: `theorem`.
- Validity domain: A device in steady state whose only source of energy is the stated input power cannot deliver more output power than it takes in (first law of thermodynamics).
- Validity conditions: none in the DSL
- Variables:
- `P_in`: Input power, the device's only energy source (W, mW, kW, MW, GW)
## Limits of application
- Assumption energy_sources: when the input is the only energy source, output power is at most the input power; when other sources exist (fuel, stored energy, ambient heat for a heat pump) this law gives no bound; the engine needs a finite number, so 1e30 times the input power stands for no bound.
- Steady state: a device may release stored energy for a short time
- Output and input are compared in the same unit of power
## References
- Callen, H. B., Thermodynamics and an Introduction to Thermostatistics, 2nd ed., Wiley (1985)
- Feynman, R. P., Leighton, R. B. and Sands, M., The Feynman Lectures on Physics, Vol. I, chapter 4, Addison-Wesley (1963)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property in `crates/limina-dsl/tests/review_laws.rs`, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound in every configuration allowed by the text is IMPOSSIBLE with the wording "inconsistent with limit power_conservation under assumptions ..."; when an assumption is not stated and one option allows the claim, the verdict is CONDITIONAL.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out, not by the DSL.
- **R-06** The built-in matcher (`crates/limina-extract/src/matcher.rs`) chooses the law by keyword and dimension; the model's limit id is a hint only.
