# Requirements: law-eddington_luminosity
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/eddington_luminosity.json`.
## Facts
- Name: Eddington luminosity. Kind: max bound. Bounded quantity dimension: `W`.
- Equation: `4*pi*G*M*m_p*c/sigma_T`
- Classification: `theoretical_bound`. Epistemic status: `model_bound`.
- Validity domain: Luminosity at which the outward radiation force on fully ionized hydrogen, by Thomson scattering on electrons, balances the inward gravitational force on the protons, for a steady, spherically symmetric flow around a mass M: L_Edd = 4 pi G M m_p c / sigma_T.
- Validity conditions: none
- Variables:
- `M`: Mass of the central object (kg, g)
## Limits of application
- Holds under the stated geometry and composition only; non-spherical accretion, other opacities or non-steady flows can give luminosities above it. Without the assumption the engine gives no verdict (needs input), and a claim above the value is inconsistent with the limit only under the spherical, steady, ionized-hydrogen assumption
- Thomson scattering only: other opacity sources change the value
- Fully ionized hydrogen: a helium-rich composition has a different mass per electron
- A bound of the model for a stable steady flow, not a limit of nature that no source can exceed
- The mass is the mass of the central object in kg or g; no solar-mass unit is in the core units table
## References
- Rybicki, G. B. and Lightman, A. P., Radiative Processes in Astrophysics, Wiley (1979)
- Frank, J., King, A. and Raine, D., Accretion Power in Astrophysics, 3rd ed., Cambridge University Press (2002)
- CODATA 2022 recommended value of the Thomson cross section, https://physics.nist.gov/cgi-bin/cuu/Value?sigmae
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or below the bound, under the stated assumption, is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim above the bound, under the assumption accretion_geometry = spherical_steady_ionized_hydrogen, is IMPOSSIBLE with the wording "inconsistent with limit eddington_luminosity under assumptions accretion_geometry=spherical_steady_ionized_hydrogen". Without the assumption, or with the other option, the engine returns NotApplicable (needs input) and no verdict.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
