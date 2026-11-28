# Requirements: limina-mcp

Status: v1. Base: `api-evaluate`, `api-limit-metadata`, `limit-author` (solve), `extract-claims` (analyze), all unchanged. New crate `limina-mcp`. `limina-core`, `limina-api`, `limina-extract` and `limina-dsl` are not modified.

## Goal

An MCP server over stdio so that an agent (Kiro) can look up the limits, check a claim and solve for a threshold through the Limina HTTP API.

## Requirements

- **M-01** The server speaks MCP over stdio, with the official Rust SDK (`rmcp`, exact version pinned). Nothing but the protocol is written to stdout.
- **M-02** It is a thin client of the Limina HTTP API. The base URL comes from `LIMINA_API_URL`, default `http://127.0.0.1:3000`. That is its only setting.
- **M-03** No verdict logic is repeated. A verdict, bound, margin or threshold in a tool result is the API's answer, passed on unchanged. A test makes the mock answer COMPATIBLE for a claim far above the bound and checks that the tool returns COMPATIBLE.
- **M-04** No key passes through it. It reads no environment variable except `LIMINA_API_URL` and sends no authorization header. A test puts a canary key in the process environment and checks that no request carries it.
- **M-05** Five tools: `list_limits`, `explain_limit(id)`, `evaluate_claim(limit_id, value, unit, constraints, parameters)`, `solve_threshold(limit_id, claim, free_parameter, constraints, parameters)`, `analyze_text(text)`. Each has a JSON input schema.
- **M-06** An API error (any status outside 200 to 299) is a tool result with `isError` true that names the status, the error code and the message. An unreachable API is reported the same way, with the URL.
- **M-07** `analyze_text` answers an explicit message when the API says `ExtractionUnavailable` (HTTP 503): extraction is unavailable on this server, and the deterministic tools still work.
- **M-08** `explain_limit` with an unknown id answers an error result that lists the known ids.
- **M-09** Tests start the real binary against a mock HTTP server and speak JSON-RPC to it.
