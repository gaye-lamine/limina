# Tasks: limina-mcp

- [x] Check that the official Rust SDK compiles with the pinned toolchain (`rmcp` 3.12.0).
- [x] `ApiClient` with typed errors.
- [x] The five tools and their input schemas.
- [x] Explicit message when extraction is unavailable.
- [x] Tests against a mock HTTP server: tool list, forwarded bodies, unchanged answers, API errors, unreachable API, no credential sent.
- [x] README section with the configuration.
- [x] `docs/mcp-demo.md`: a recorded stdio session.
- [x] Register the server in `.kiro/settings/mcp.json` and call each tool from Kiro. Done: the five tools were called from a Kiro session and their raw tool entries and results are in `docs/mcp-demo.md`. The registration file itself is local and is not committed.
