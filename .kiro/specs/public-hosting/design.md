# Design: public-hosting

## Modules (crate `limina-api`)
- `limiter.rs`: `IpRateLimiter` (sliding window per key in a `Mutex<HashMap<String, VecDeque<Instant>>>`, bounded by `MAX_TRACKED`), `parse_limit`, `parse_flag`, `forwarded_ip`, and the middleware `limit_llm_routes`. The middleware reads `ConnectInfo<SocketAddr>` when present (the server serves with `into_make_service_with_connect_info`); a test that calls the router with `oneshot` has none and counts as "unknown". The 429 is `ApiError::TooManyRequests { retry_after_secs }` (code `RateLimited`, the same code as the extractor's own per-minute limit, so the web app needs one case) plus `Retry-After`.
- `static_files.rs`: `with_static_dir(router, dir)` sets the router's fallback; `safe_relative` rejects anything but plain names; `read_inside` canonicalizes both the folder and the file and requires the prefix, so a link cannot leave the folder. No new dependency (`tokio` gains its `fs` feature).
- `lib.rs`: `router_with` puts the three model routes in their own router with `route_layer(from_fn_with_state(...))` and merges it; `AppState` gains `limiter` and `MAX_AUTHORED_LIMITS`.
- `main.rs`: `listen_address(addr, port)` (tested), static folder from `STATIC_DIR`.
- `limina-extract/src/author.rs`: `normalize_dimensions` and `normalize_parameters` run between the draft forcing and the schema parsing; `hint_for` adds a fixed hint to a repair message; `EXAMPLE` and the rules of the prompt.

## Decisions
- D-01 The limiter is in `AppState`, not global: tests build as many states as they need.
- D-02 In memory only: a restart empties the counters and the drafts. For several instances, a shared store would be needed; out of scope for a demo.
- D-03 Own static handler instead of `tower-http`: about 100 lines, no dependency to pin, and the `/v1/` rule fits in it.
- D-04 A local run stays on the loopback unless `PORT` or `LIMINA_API_ADDR` says otherwise.
- D-05 The Forge fixes slips that cannot change a law's meaning (a field the schema forbids, a notation of the same dimension). It does not rename a parameter that collides with a constant, and it does not repair a formula: those stay refusals sent back to the model.
- D-06 Measured on the configured chain (2026-11-27): before, three real sentences failed after three attempts with a schema error; after, six of six were written (one to three attempts). The remaining refusals are the model's dimension mistakes, caught by the check.

## Verification
- `crates/limina-api/tests/hosting.rs`: the sixteenth request is 429, the three routes share a budget, deterministic routes are free, proxy headers count only when trusted, no limiter without a request for one, the web app and its fallback, traversal attempts.
- `crates/limina-api/tests/authoring.rs`: the 51st draft evicts the first; a wrong `kind` is accepted at the first attempt.
- Unit tests in `limiter.rs`, `static_files.rs`, `main.rs` and `author.rs`.
- Smoke test of the release binary with `PORT`, `STATIC_DIR`, `TRUST_PROXY` and `LLM_RATE_LIMIT_PER_HOUR=3`.
