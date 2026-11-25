# Requirements: law-stefan_boltzmann
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/stefan_boltzmann.json`.
## Facts
- Name: Stefan-Boltzmann limit. Kind: max bound. Bounded quantity dimension: `W/m^2`.
- Equation: `sigma_SB*T^4`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Power per unit area emitted into the half space by the surface of a body at uniform absolute temperature T, in vacuum or a non-absorbing medium.
- Validity conditions: T > 0
- Variables:
- `T`: Absolute temperature of the emitting surface (K)
## Limits of application
- The bound is reached by a black body; a real surface has emissivity at most 1 (Kirchhoff's law at thermal equilibrium), so its flux is lower
- Total emitted flux over all wavelengths; absorbed or reflected ambient radiation is not counted
- Far-field, macroscopic surfaces: not valid at distances or sizes comparable to the thermal wavelength
## References
- Kittel, C. and Kroemer, H., Thermal Physics, 2nd ed., W. H. Freeman (1980)
- Planck, M., The Theory of Heat Radiation, P. Blakiston's Son & Co. (1914)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound is IMPOSSIBLE with the wording "inconsistent with limit stefan_boltzmann under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
