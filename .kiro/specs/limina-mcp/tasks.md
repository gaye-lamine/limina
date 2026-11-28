# Tasks: limina-mcp

- [x] Check that the official Rust SDK compiles with the pinned toolchain (`rmcp` 3.12.0).
- [x] `ApiClient` with typed errors.
- [x] The five tools and their input schemas.
- [x] Explicit message when extraction is unavailable.
- [x] Tests against a mock HTTP server: tool list, forwarded bodies, unchanged answers, API errors, unreachable API, no credential sent.
- [x] README section with the configuration.
- [x] `docs/mcp-demo.md`: a recorded stdio session.
- [ ] Register the server in `.kiro/settings/mcp.json` and call each tool from Kiro. Not done in the session that built the crate: writing under `.kiro/settings/` was refused by a permission rule.
