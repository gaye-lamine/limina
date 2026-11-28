# Design: limina-mcp

## Layout

- `src/client.rs`: `ApiClient` (reqwest, no `json` feature, 90 s timeout). `get` and `post` return `ApiReply { status, body }` or a typed `ClientError` (`Build`, `Unreachable`, `NotJson`).
- `src/server.rs`: `LiminaMcp` with the five tools (`#[tool_router]`, `#[tool_handler]` from `rmcp`). Input structs derive `Deserialize` and `schemars::JsonSchema`; the field docs are the schema descriptions.
- `src/main.rs`: reads `LIMINA_API_URL`, serves on `rmcp::transport::stdio`, logs one line to stderr.

## Mapping

| Tool | Request | Result |
| --- | --- | --- |
| `list_limits` | `GET /v1/limits` | `{count, limits}` with seven fields kept per limit |
| `explain_limit` | `GET /v1/limits` | the entry with the id, whole |
| `evaluate_claim` | `POST /v1/evaluate` with `{limit_id, claim: {value, unit, uncertainty?}, constraints, parameters}` | the API body |
| `solve_threshold` | `POST /v1/solve` with `{limit_id, claim, free_parameter, constraints, parameters}` | the API body |
| `analyze_text` | `POST /v1/analyze` with `{text}` | the API body, or the explicit message on 503 `ExtractionUnavailable` |

The result is one text block holding the pretty-printed JSON. A failure is the same block with `isError` true.

## Decisions

- The SDK exists (`rmcp` 3.12.0, `rust-version` 1.88, the toolchain is 1.100) and compiles with the pinned versions, so the protocol is not written by hand.
- The server is not given a model key and has no code path to one. Extraction stays in the API.
- `list_limits` drops the label maps and references to keep the list short; `explain_limit` returns the whole entry.

## Registration

`.kiro/settings/mcp.json` runs `cargo run --quiet -p limina-mcp` with `LIMINA_API_URL`. The README gives the exact file.
