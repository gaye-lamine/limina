# Requirements: law-rayleigh_criterion
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/rayleigh_criterion.json`. Added after the 26/11 review of a multi-domain image.
## Facts
- Name: Diffraction limit of a circular aperture. Kind: min bound. Bounded quantity dimension: `rad`.
- Equation: `1.22*wavelength/D`
- Option `resolution_criterion = rayleigh`: `1.22*wavelength/D`
- Option `resolution_criterion = sparrow`: `0.947*wavelength/D`
- Classification: `theoretical_bound`. Epistemic status: `model_bound`.
- Validity domain: Smallest angular separation a diffraction-limited telescope or camera with a single circular aperture of diameter D resolves at wavelength lambda: 1.22 lambda / D by the Rayleigh criterion, 0.947 lambda / D by the stricter Sparrow criterion.
- Validity conditions: none in the DSL
- Variables:
- `wavelength`: Wavelength of the light (m, um, nm)
- `D`: Diameter of the circular aperture (m, cm, mm)
## Limits of application
- Assumption resolution_criterion: unstated, the engine takes both; a claim below the Sparrow limit is inconsistent under both criteria, a claim between the two is possible only with the Sparrow criterion.
- Single filled circular aperture: interferometric arrays (baseline instead of diameter), speckle and lucky imaging, and super-resolution post-processing are outside its assumptions
- Diffraction only: atmospheric seeing and optical aberrations make real resolution worse, never better
## References
- Rayleigh, Lord, Investigations in optics, with special reference to the spectroscope, Philosophical Magazine 8(49), 261-274 (1879), doi:10.1080/14786447908639684
- Sparrow, C. M., On Spectroscopic Resolving Power, Astrophysical Journal 44, 76-86 (1916), doi:10.1086/142271
- Born, M. and Wolf, E., Principles of Optics, 7th ed., Cambridge University Press (1999)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property in `crates/limina-dsl/tests/review_laws.rs`, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound in every configuration allowed by the text is IMPOSSIBLE with the wording "inconsistent with limit rayleigh_criterion under assumptions ..."; when an assumption is not stated and one option allows the claim, the verdict is CONDITIONAL.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out, not by the DSL.
- **R-06** The built-in matcher (`crates/limina-extract/src/matcher.rs`) chooses the law by keyword and dimension; the model's limit id is a hint only.
