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
