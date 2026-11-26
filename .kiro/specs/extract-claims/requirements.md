# Requirements: extract-claims

Status: v1, frozen. Base: `api-evaluate` v1 as amended by `api-limit-metadata` v1; `decision-engine` v5, `limit-betz`, `limit-carnot`, `limit-cop` (all unchanged). New crate `limina-extract`; endpoints added in `limina-api`. `limina-core` is not modified.

## Summary

A user pastes a paragraph. A language model reads the claims out of it as structured JSON. Deterministic code then checks that every value is written in the text, normalizes units, and sends each claim to the same evaluation code as `POST /v1/evaluate`. The verdict is always computed by the engine; the language model never decides anything (product rule).

## Facts

- Provider: one implementation of an `ExtractionProvider` trait for an OpenAI-compatible chat completions endpoint over HTTP. No provider-specific code outside it (tech rule).
- Configuration from environment variables only: `NVIDIA_API_KEY` (secret), `LIMINA_LLM_MODEL` (default `nvidia/nemotron-3.5-lightning-30b-a3b`), `LIMINA_LLM_BASE_URL` (default `https://integrate.api.nvidia.com/v1`), `LIMINA_LLM_DISABLE_THINKING` (default true: send `chat_template_kwargs {"enable_thinking": false}`).
- Request: temperature 0, `max_tokens` 600, timeout 15 s; one retry with backoff on HTTP 429 or 5xx; at most 30 requests per minute in the process; in-memory cache keyed by the SHA-256 of the normalized text.
- Output schema (strict): `{"claims":[{"limit_id": string|null, "quantity": string, "value": number, "unit": string, "evidence": string, "constraints": {assumption_id: option_id}, "parameters": {name: {"value": number, "unit": string, "evidence": string}}}], "missing": [string]}`.

## Security requirements (tests written first)

- **S-01 Secret key.** `NVIDIA_API_KEY` is never logged, never written to a file, never returned by the API, and never part of an error message or a `Debug` output.
- **S-02 Traceability.** Every evidence must be an exact substring of the input text, and every numeric value must appear in its evidence. Otherwise the field is dropped and flagged "not traceable to the text". The engine never receives an untraceable value.
- **S-03 Prompt injection.** The text is passed as data inside a delimited block; the delimiters are neutralized inside the text; the system prompt says instructions inside the text are ignored. Whatever the text says, the schema is unchanged and every verdict comes from the engine.

## Functional requirements

- **R-01** Read only `message.content`, never `reasoning_content`. Empty content, `finish_reason` "length", non-JSON output, or JSON that does not match the schema is `ExtractionFailed`. Markdown code fences are stripped, then the first JSON object is parsed.
- **R-02** The system prompt is built at runtime from the limits metadata: limit ids, parameter names with allowed units, assumption ids with option ids.
- **R-03** A constraint is kept only if the text states it explicitly; otherwise the assumption stays unconstrained (COP without a stated mode gives CONDITIONAL).
- **R-04** Unit normalization: "%" becomes a dimensionless fraction (value / 100) with a visible note ("60 % becomes 0.60"). A unit not allowed for the limit is not guessed: the claim is returned flagged "unit not understood" and is not evaluated.
- **R-05** A hot and cold temperature pair is never reordered; the engine decides.
- **R-06** `GET /v1/capabilities` returns `{"extraction": bool}`, false when `NVIDIA_API_KEY` is not set.
- **R-07** `POST /v1/analyze {"text"}` (at most 5000 characters) extracts, checks, normalizes, then evaluates each claim with the same code path as `POST /v1/evaluate`. Per claim: extracted fields, evidence with character offsets (start, end), normalization notes, flags, and either the evaluation result or `needs_input` with a reason (Betz `NotApplicable` gives "reference area not stated"). Also: model id, cached, milliseconds. Never the key, never the raw model output.
- **R-08** Without extraction, `POST /v1/analyze` returns 503 `ExtractionUnavailable`. `POST /v1/evaluate` never calls a language model.

## Acceptance criteria

Mock-provider tests (no network): valid JSON; JSON in code fences; empty content; `finish_reason` length; value not in the text; evidence not a substring; percent normalization; unit not understood; hot and cold swapped; prompt injection; 429 then success; cache hit; rate limit; capabilities with and without a key; sentinel key never in any log or response. End to end through the handlers with the mock:
1. "heat engine between 600 K and 300 K at 60% efficiency" gives IMPOSSIBLE for Carnot with bound 0.5.
2. "power coefficient of 0.70 with a shrouded rotor" gives `needs_input` for Betz ("reference area not stated").
3. "heat pump COP of 14.5 between 300 K and 280 K" gives CONDITIONAL for COP.

One live test, `#[ignore]`, run only if `NVIDIA_API_KEY` is set, on sentences 1 and 2.

## Out of scope

Any change to `limina-core` or its specs; web user interface for analysis; persistence; authentication of API callers.

## Revision 2026-11-26 (audit of 26/11)
- **R-09** Carnot never applies to a converter that is not a heat engine. When the paragraph sent to `POST /v1/analyze` names one (fuel cell, electrochemistry, battery, solar cell, photovoltaic cell, electrolyser; English and French words, `matcher::NON_THERMAL`), the matcher never chooses `carnot`, by keyword or by the model's hint.
- **R-10** The model's `carnot` hint alone (no keyword match) is kept only when the paragraph states a heat engine cycle (`matcher::HEAT_ENGINE`: heat engine, engine, turbine, steam, power plant, hot or cold reservoir, and their French forms). Otherwise the hint is dropped, with a note.
- **R-11** The assumption `conversion_process` of the served Carnot law is decided by the code, never by the model: `heat_engine_cycle` when the paragraph states a heat engine cycle and names no other converter, left open otherwise. Left open, the engine answers CONDITIONAL for a claim beyond the bound, never IMPOSSIBLE.
- **R-12** The context words of a law (`matcher::CONTEXT`, for example "escapes") and the named body with "surface" (`fill_body_constants`) are looked for in the whole paragraph. Parameters read from the text without the model (`fill_parameters_from_sentence`) and options (`fill_options_from_sentence`) stay in the claim's sentence, so the numbers of two claims are never mixed.
- **AC** API mock tests in `crates/limina-api/tests/multidomain.rs`: the fuel cell sentence, French and English, with and without the Carnot hint, and with Carnot named by the law-naming step, is never IMPOSSIBLE; temperatures without a heat engine give CONDITIONAL; a stated heat engine stays IMPOSSIBLE; an escape stated in the previous sentence selects `escape_speed_min` (IMPOSSIBLE at 8 km/s); a body named in the previous sentence gives its constants; a launch speed with no escape stated is not checked as an escape. A launch speed alone does not claim an escape: checking it against the escape speed would be a false IMPOSSIBLE.
- **R-13** A dimensionless parameter the model left out (`aperture`, `snr`) is read from the claim's sentence only when a phrase that names it is right next to its number ("numerical aperture of 1.4", "NA = 0.95", "1.3 NA", "ouverture numérique de 1,4"; at most one link word: of, =, :, is, de), the number has no unit after it and is not used elsewhere, the value meets the law's condition (aperture > 0, snr >= 0), and exactly one number qualifies. A bare number ("Model 2000") is never taken. A period closing an initialism ("N.A.", "e.g.") does not end the sentence.
- **R-14** The parameter pass of the law pipeline applies R-02's unit rule too: the unit written next to the number in the text wins over the unit read, and a value whose written unit the law does not allow is rejected ("details of 100 nm" read as a momentum of 100 kg*m/s gives no value).
- **AC** Unit tests in `crates/limina-extract/src/check.rs` (aperture read when the model omits it, four variants; bare number, distant phrase, NA = 0, two values and "Arena 1.4" rejected; initialism) and `crates/limina-extract/tests/law.rs`; API test `a_parameter_read_with_a_unit_the_text_does_not_write_is_rejected`.

## Revision 2026-11-26 (English scientific units)
- **R-15** Units the core parser does not know are converted on reading by `crates/limina-extract/src/units.rs`, with a note "X unit becomes Y core-unit"; `limina-core` is unchanged. Factors are definitions or core constants: °C to K (T + 273.15, absolute temperatures only: never when the quotation or the words before the number speak of a difference, rise, drop, change or Δ); bps/kbps/Mbps/Gbps to bit/s units; arcsec, arcmin, deg to rad; Å and angstrom to m; solar mass (IAU 2015 nominal GM_sun over the CODATA 2022 G) to kg; solar luminosity (3.828e26 W) to W; yr, year(s), kyr, Myr, Gyr (Julian year) to s; light-year (c × Julian year), pc, kpc, Mpc, Gpc (648000/π au) to m; km/s/Mpc to Hz; u, Da, amu (CODATA 2022 m_u) to kg. They apply to claims, to parameters given by the model, to parameters read from the sentence, to the parameter pass of the law pipeline, and to the matcher's dimension check. °F and any other symbol are not converted.
- **R-16** The laws' unit lists accept the core units texts use: au (kepler_third a, newton_gravity r, escape_velocity r), t (tsiolkovsky m0, mf), us, µs, ns (time_dilation tau, faraday_emf dt), um, nm, pm (coulomb r), g/cm^3 (friedmann_flat rho). Temperatures stay K in the definitions (the DSL accepts K only); °C reaches them through R-15.
- **R-17** R-13 also covers `n_turns` ("turns", "coil turns", "number of turns", "windings"; a whole number >= 1), `n` ("level", "quantum number", "n ="; a whole number >= 1) and `n0` ("initial nuclei", "initial atoms", "N0", "N0 ="; > 0). A symbol link may touch its phrase ("turns: 150").
- **AC** Unit tests in `units.rs` (each factor, a Celsius difference never converted, core and unknown units untouched) and `check.rs` (turns, level, N0 and their rejections; a parameter in solar masses read from the sentence); API tests in `crates/limina-api/tests/realistic_english.rs`: steam turbine in °C (IMPOSSIBLE at 60 %, COMPATIBLE at 40 %), Landauer at 27 °C, telescope in arcsec (IMPOSSIBLE, CONDITIONAL, COMPATIBLE), Heisenberg, de Broglie, muon lifetime in µs, black hole in solar masses, orbit in au and years, Hubble constant in km/s/Mpc, modem in Mbps, coil turns, hydrogen level, decay in years with N0; and `analyze.rs` (°C converted with a note; a °C difference never taken as a temperature).

- **R-18** A parameter takes any unit of the physical dimension its law declares, not only the listed units: a core unit as written (the engine converts it), or a unit of R-15 converted first. Kept apart although the dimension is the same: bit, rad and sr (all dimension 1 in the core) from each other and from a plain ratio; a plain ratio takes only "dimensionless", so dB is never a linear ratio; an absolute temperature takes K only, °C and °F being converted to K first and a temperature difference never. R-15 also converts, with NIST SP 811 exact factors: °F to K ((T - 32) × 5/9 + 273.15), psi, mbar, Torr, mmHg to Pa, hp (550 ft·lbf/s) to W, BTU (IT), erg, cal and kcal (thermochemical) to J, erg/s to W, in, ft, yd, mi and their words to m, mph and knots to m/s, lb and oz to kg, gauss to T. "G" (the gravitational constant) and "Gs" (the SI gigasecond) are never the gauss; "in" is read as inches only when the model gives it, never after a number of the text ("3 in 10").
- **AC** `units.rs` (every NIST factor, a Fahrenheit difference never converted, "Gs" and "in" not taken), `check.rs` (any unit of the dimension; dB, rad/sr/bit, mK and a temperature drop refused), `realistic_english.rs` (steam turbine in °F, generator in hp, escape velocity with a radius in miles, energy in erg).