# Requirements: law-shannon_hartley
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/shannon_hartley.json`.
## Facts
- Name: Shannon-Hartley limit. Kind: max bound. Bounded quantity dimension: `bit/s`.
- Equation: `B*ln(1 + snr)/ln2`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Error-free information rate over a band-limited channel of bandwidth B with additive white Gaussian noise and a signal-to-noise power ratio snr (a plain ratio, not decibels).
- Validity conditions: none in the DSL
- Variables:
- `B`: Channel bandwidth (Hz, kHz, MHz, GHz)
- `snr`: Signal-to-noise power ratio, linear (not dB) (dimensionless)
## Limits of application
- Additive white Gaussian noise channel; other noise or fading channels have other capacities
- snr is the linear power ratio over the bandwidth B; a value in dB must be converted first
- Capacity of the channel, reached only with ideal coding: a link does not reach it
- Defined for a positive bandwidth and a non-negative ratio snr; the core checks no condition on them here
- Multiple antennas or users (MIMO) are not covered
## References
- Shannon, C. E., A Mathematical Theory of Communication, Bell System Technical Journal 27(3), 379-423 (1948), doi:10.1002/j.1538-7305.1948.tb01338.x
- Cover, T. M. and Thomas, J. A., Elements of Information Theory, 2nd ed., Wiley (2006)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound is IMPOSSIBLE with the wording "inconsistent with limit shannon_hartley under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
