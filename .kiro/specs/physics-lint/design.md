# Design: physics-lint
Status: v1, frozen. Based on `requirements.md` v1.

## 1. limina-api (additions)
- `analyze::analyze_text(&AppState, &str) -> Result<AnalyzeResponse, ApiError>`: length and blank checks, extraction, per-claim evaluation. The `POST /v1/analyze` handler parses the body and calls it; its contract is unchanged.
- `authoring::solve_with(&AppState, &SolveRequestDto) -> Result<SolveResponse, ApiError>`: the body of `POST /v1/solve`, which now calls it.

## 2. crates/limina-cli
Dependencies: `limina-api`, `limina-extract` (paths), `serde`, `serde_json`, `tokio` (pinned as in the workspace). No argument-parsing or dotenv crate.
- `args.rs`: `parse(&[String]) -> Result<Command, String>`.
- `env.rs`: `.env` parser (`KEY=VALUE`, `#` comments, optional `export `, quotes trimmed); the lookup prefers the process environment.
- `position.rs`: `line_col(text, char_offset)`; lines split on `\n`, so a `\r` before it never moves a column.
- `check.rs`: the file is split into paragraphs (blocks separated by blank lines); paragraphs without a digit are skipped; each other paragraph goes to `analyze_text`. Reason: the extraction answer is capped at 600 tokens (extract-claims), so a whole document with several claims can be cut off; one paragraph keeps each answer short and each text under 5000 characters. Offsets are shifted by the paragraph start. A claim without evidence takes the paragraph start. A paragraph whose answer is unusable (`ExtractionFailed`: no JSON, cut at the token limit, not the schema) is asked again, at most 3 answers in all; a paragraph still failing is listed in `errors` at its line and the check goes on; no key, a provider failure or the rate limit stop it, since every next call would fail too. A retry asks again for the same text and never touches a value or a verdict. `missing` is concatenated; `extraction.cached` is true only if every call was cached; `ms` is summed.
- `settle.rs`: from the limit metadata (`registry::list()`): assumptions with more than one option that the claim does not constrain (CONDITIONAL, `needs_input` with `NotApplicable`), missing parameters (INSUFFICIENT_DATA, `Depends`), with their labels.
- Inverse line: for IMPOSSIBLE and CONDITIONAL, each declared parameter with a value is tried as the free parameter in declaration order, the others fixed; the first `solve_with` success gives its sentence. Failures are silent.
- `render.rs`: text and JSON.
- `run.rs`: `run(args, config, make_state) -> Outcome { code, stdout, stderr }`, so tests inject a mock provider; `main.rs` builds the config from the environment and `.env`.

## 3. Hook
Created with Kiro's `createHook` tool. Its action types are `command` (stdout is forwarded to the session only for SessionStart, UserPromptSubmit, PreTaskExec, PreCompact and PostCompact, not for PostFileSave) and `agent` (appends a static prompt). The hook is therefore an `agent` action: the prompt asks the agent to run `limina check` on the saved file, show the report, and for each finding propose to state the missing assumption or to ask the author, never to change a number.

## 4. Tests
`crates/limina-cli/tests/check.rs` with `MockProvider`; `tests/live.rs` `#[ignore]`; unit tests in `position.rs`, `env.rs`, `args.rs`.
