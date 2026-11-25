# Requirements: law-radioactive_decay
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/radioactive_decay.json`.
## Facts
- Name: Radioactive decay law. Kind: equality (predicted value, relative tolerance 0.01). Bounded quantity dimension: `dimensionless`.
- Equation: `n0*exp(-t*ln2/t_half)`
- Classification: `empirical_law`. Epistemic status: `model_bound`.
- Validity domain: Expected number N(t) = N0 exp(-t ln2 / T_half) of undecayed nuclei of one isotope after a time t, from N0 nuclei at t = 0.
- Validity conditions: t >= 0, t_half > 0, n0 >= 0
- Variables:
- `n0`: Number of nuclei at t = 0 (dimensionless)
- `t`: Elapsed time (s, min, h, d)
- `t_half`: Half-life of the isotope (s, min, h, d)
## Limits of application
- Statistical law: an actual count fluctuates around the expected value (about the square root of N for a Poisson process), so a small sample can lie outside the 1 % band without a contradiction
- The half-life is a parameter of the isotope, it is not provided by the law and is never assumed
- Single isotope with no feeding from a parent: decay chains need the Bateman equations
- Time units are s, min, h and d; a half-life in years must be converted by the user
- Holds for a constant decay probability per unit time, which is the case for ordinary decay modes of free nuclei
## References
- Rutherford, E. and Soddy, F., The cause and nature of radioactivity, Part I, Philosophical Magazine 4(21), 370-396 (1902), doi:10.1080/14786440209462856
- Krane, K. S., Introductory Nuclear Physics, 3rd ed., Wiley (1988)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit radioactive_decay under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
