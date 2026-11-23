# Design: extract-claims

Status: v1, frozen. Based on `requirements.md` v1.

## 1. Crate `limina-extract`

No dependency on `limina-core` or `limina-api`: the API passes the limits metadata in as a `Catalog`.

Pinned dependencies: `reqwest =0.13.5` (default features off, `rustls`), `sha2 =0.11.1`, `serde =1.0.229`, `serde_json =1.0.154`, `thiserror =2.0.21`, `tokio =1.54.1` (`time`, `sync`).

Modules:

- `config.rs`: `ExtractConfig { model, base_url, disable_thinking, api_key: Option<SecretKey> }`, built by `from_lookup(fn(&str) -> Option<String>)` so tests never touch the process environment; `from_env()` calls it with `std::env::var`. `SecretKey` has no `Display`, no `Serialize`, and a `Debug` that prints `[redacted]`.
- `provider.rs`: trait `ExtractionProvider { fn model(&self) -> &str; fn complete(&self, ChatRequest) -> BoxFuture<Result<ChatReply, ProviderError>> }`. `ChatReply { content: Option<String>, finish_reason: Option<String> }`. `ProviderError { Status(u16), Timeout, Transport }`, never carrying the body, the headers or the key.
- `openai.rs`: `OpenAiProvider`, the only provider-specific code. POST `{base_url}/chat/completions` with bearer key, temperature 0, `max_tokens` 600, `chat_template_kwargs {"enable_thinking": false}` when thinking is disabled, timeout 15 s. Reads `choices[0].message.content` and `finish_reason` only.
- `catalog.rs`: `Catalog { limits: Vec<CatalogLimit { id, claim_unit, parameters: [(name, units)], assumptions: [(id, options, option labels)] }> }`.
- `prompt.rs`: `system_prompt(&Catalog)` and `user_message(text)`. The text is wrapped in `<<<TEXT` / `TEXT>>>`; any occurrence of either marker inside the text is replaced by a neutral `[marker removed]` before wrapping.
- `parse.rs`: strip code fences, find the first balanced JSON object (string-aware), deserialize into `ModelOutput` with `deny_unknown_fields`.
- `check.rs`: deterministic checks, no model. Evidence located with `str::find`, offsets converted to character indices. Numbers in the evidence are scanned (sign, digits, one decimal point or comma, exponent) and compared to the value with a relative tolerance of 1e-9. Unit normalization and constraint filtering. Output: `CheckedClaim` with `notes` and `flags`.
- `extractor.rs`: `Extractor { provider, catalog, cache, limiter, retry_backoff }`. `extract(text)`: cache lookup on SHA-256 of the normalized text (trimmed, runs of whitespace collapsed); otherwise rate limit check (30 per 60 s sliding window, `RateLimited` beyond), call with one retry on 429 or 5xx after the backoff, parse; the parsed model output is cached, and the deterministic checks always run against the actual text.
- `error.rs`: `ExtractError { ExtractionFailed(&'static str reason), RateLimited, Provider(ProviderError) }`.

## 2. Deterministic rules

- Claim value: kept only if its evidence is a substring of the text and the value appears in it; else dropped, flag "not traceable to the text", claim not evaluated.
- Parameter: same rule; an untraceable parameter is dropped (the engine then reports it missing).
- Claim unit: the limit's `claim_unit`; "%" allowed for a dimensionless claim, divided by 100, note "60 % becomes 0.60". Anything else: flag "unit not understood", not evaluated. Parameter units: only those listed by the metadata; anything else, including "°C", flags the claim "unit not understood".
- Constraint: kept only if the assumption and option exist for the limit and the option, written as its display label or its id with spaces, appears in the text (case-insensitive); else dropped with flag "constraint not stated in the text". This makes R-03 checkable without trusting the model.
- Unknown or null `limit_id`: flag "limit not recognized", not evaluated.
- Temperatures are passed as given; the engine reports `InvalidTemperatureOrder`.

## 3. API

- `AppState { extractor: Option<Arc<Extractor>> }`; `router_with(state)`, and `router()` keeps today's behaviour with no extractor, so `POST /v1/evaluate` cannot reach a model.
- The evaluation code of `POST /v1/evaluate` moves into `handlers::evaluate_request(&EvaluateRequest) -> Result<EvaluateResponse, ApiError>`, used by both endpoints.
- `GET /v1/capabilities`: `{"extraction": state.extractor.is_some()}`.
- `POST /v1/analyze`: body `{"text"}` (`deny_unknown_fields`), at most 5000 characters (`TextTooLong`, 422), not blank (`EmptyText`, 422). 503 `ExtractionUnavailable` without extractor; `ExtractionFailed` 502; `RateLimited` 429; provider errors 502 `ExtractionProviderError`. Per claim `status` is `evaluated` (with `result`) or `needs_input` (with `reason` and, for an engine refusal, `code`). `NotApplicable` maps to "reference area not stated".
- `main.rs` builds the extractor only when `NVIDIA_API_KEY` is set.

## 4. Tests

Unit tests in each module; security tests first (`tests/security.rs` in both crates); handler tests with a mock provider; one `#[ignore]` live test in `limina-api/tests/analyze_live.rs`.
