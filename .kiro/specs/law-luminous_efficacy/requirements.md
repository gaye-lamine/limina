# Requirements: law-luminous_efficacy
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/luminous_efficacy.json`.
## Facts
- Name: Luminous efficacy of radiation. Kind: max bound. Bounded quantity dimension: `lm/W`.
- Equation: `Kcd`
- Classification: `theoretical_bound`. Epistemic status: `theorem`.
- Validity domain: Luminous efficacy of radiation (lumens per watt of optical power) of any light, for a standard photopic observer.
- Validity conditions: none checked by the core
- Variables:
- none (the bound is a constant)
## Limits of application
- Efficacy of the radiation itself, not of a lamp: the electrical efficacy of a source is lower because of conversion losses
- The bound Kcd is reached only by monochromatic light at 540 THz; white light has a lower maximum, which is not checked here because no verified reference is included
- Standard photopic observer (CIE): scotopic and mesopic vision have other efficacies
- The bound is the definitional value 683 lm/W; the photopic maximum differs from it by a few parts per million, so a decision within that margin is not meaningful
## References
- BIPM, The International System of Units (SI), 9th edition (2019), definition of the candela, Kcd = 683 lm/W at 540e12 Hz
- CODATA 2022 value of the luminous efficacy Kcd (exact in the 2019 SI), https://physics.nist.gov/cgi-bin/cuu/Value?kcd
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound is IMPOSSIBLE with the wording "inconsistent with limit luminous_efficacy under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
