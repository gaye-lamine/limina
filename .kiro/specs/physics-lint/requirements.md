# Requirements: physics-lint
Status: v1, frozen. Base: `extract-claims` v1, `limit-author` v1, `api-evaluate` v1 as amended (all unchanged). New crate `limina-cli`; `limina-api` gets additions only. `limina-core` is not modified.

## Summary
A command `limina check <file>` reads a text or markdown file, extracts the claims and evaluates them with the same code as `POST /v1/analyze`, and prints one block per claim at its position in the file, like a compiler. A Kiro hook runs it when a markdown file under `docs/` or `demo/` is saved. The verdict is always computed by the engine.

## Requirements
- **R-01 Command.** Binary `limina`, `limina check <file> [--format text|json]`, default `text`. Unknown arguments print the usage and exit 2.
- **R-02 Same code path.** Extraction, checks and evaluation go through one public function of `limina-api` that `POST /v1/analyze` also uses. The inverse line goes through one public function that `POST /v1/solve` also uses. The CLI has no verdict logic.
- **R-03 Key.** The key is read from the process environment, else from `.env` in the current directory. Without a key: a clear message on stderr, exit 2. The key is never printed.
- **R-04 Text output.** One block per claim: `file:line:col: <severity>: <VERDICT> <limit>`; then the claim and the bound; then the API wording (or the reason for `needs_input`); for CONDITIONAL, INSUFFICIENT_DATA and `needs_input` a line `to settle this, state: ...` naming the unconstrained assumptions with their options and the missing parameters; for IMPOSSIBLE and CONDITIONAL, where the solver answers, a line with the solver sentence. A summary line ends the report.
- **R-05 Position.** Line and column (1-based, in characters, UTF-8 safe) come from the evidence offsets returned by the analysis.
- **R-06 Exit codes.** 0: no IMPOSSIBLE. 1: at least one IMPOSSIBLE. 2: analysis impossible (no key, unreadable file, network failure, extraction failed, text too long); 2 wins over 1. A paragraph that cannot be analysed is reported at its line and the other paragraphs are still checked, unless the error would repeat (no key, provider failure, rate limit). CONDITIONAL, INSUFFICIENT_DATA and `needs_input` are warnings and never give 1.
- **R-07 JSON output.** The fields of the `POST /v1/analyze` response; each claim also has `file`, `line`, `column`, `to_settle` and `inverse`; the report has `file` and `errors` (paragraphs not analysed).
- **R-08 Demo.** `demo/spec-heat-pump.md`, marked illustrative, with four numeric claims: one IMPOSSIBLE, one CONDITIONAL (COP without a stated mode), one COMPATIBLE, one `needs_input`.
- **R-09 Hook and steering.** A Kiro hook `physics-lint` (PostFileSave, markdown files under `docs/` or `demo/`) created with Kiro's hook tool; the agent may propose to state a missing assumption or to ask the author, never to change a number to make a claim pass. Steering `.kiro/steering/physics-claims.md` (fileMatch `docs/**`, `demo/**`).

## Acceptance criteria
1. Offsets to line and column on ASCII, multi-byte and CRLF text.
2. With the mock provider, no network: exit 0, 1 and 2; text and JSON output; no key gives exit 2; the key never printed; the demo gives exactly one IMPOSSIBLE, one CONDITIONAL, one COMPATIBLE and one `needs_input`, and the Carnot claim shows "Th is at least 750 K".
3. One `#[ignore]` live test using the key in `.env`.
