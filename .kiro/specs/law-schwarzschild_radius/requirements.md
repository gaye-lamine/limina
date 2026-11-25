# Requirements: law-schwarzschild_radius
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/schwarzschild_radius.json`.
## Facts
- Name: Schwarzschild radius. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `m`.
- Equation: `2*G*M/c^2`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Radius of the event horizon of a non-rotating, uncharged mass M in vacuum (Schwarzschild solution of general relativity): r_s = 2 G M / c^2. The claim is the radius in the Schwarzschild coordinate, the circumference divided by 2 pi.
- Validity conditions: none
- Variables:
- `M`: Mass of the body (kg, g)
## Limits of application
- Non-rotating and uncharged: a rotating (Kerr) or charged black hole has a different horizon radius
- The formula gives the horizon radius for a black hole; it does not say that a given body is one, and it is not the radius of a star or planet
- The uncertainty of G (CODATA 2022) is propagated by the engine, apart from the tolerance
- The mass is in kg or g; no solar-mass unit is in the core units table
## References
- Schwarzschild, K., On the gravitational field of a mass point according to Einstein's theory (English translation by S. Antoci and A. Loinger of Sitzungsberichte der Koeniglich Preussischen Akademie der Wissenschaften 1916, 189-196), arXiv:physics/9905030, doi:10.48550/arXiv.physics/9905030
- Misner, C. W., Thorne, K. S. and Wheeler, J. A., Gravitation, W. H. Freeman (1973)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim equal to the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit schwarzschild_radius under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
