# Requirements: web-evaluate

Status: v1, frozen. Based on `api-evaluate` v1 (`crates/limina-api`). The API and `crates/` are not modified by this spec.

## Scope

A new Angular app in `web/` with one page "Evaluate" and a "Limits" section. It calls `GET /v1/limits` and `POST /v1/evaluate` and shows what the API returns. It computes no verdict. No extraction, no LLM, no authentication, no end-to-end tests.

## Requirements

- R-01 The app is a standalone-component Angular app using signals and strict TypeScript. Versions are pinned exactly (see `design.md`).
- R-02 The "Limits" section lists the limits from `GET /v1/limits`: id, direction, assumptions with their options, parameters with kind and required flag. A failed request shows the error in the bordered error block.
- R-03 The "Evaluate" form adapts to the selected limit. It has: claim value, claim unit, optional uncertainty, one selector per assumption (options from the limit metadata, plus "not specified"), and one value and unit field per parameter the limit declares.
- R-04 The request follows the API DTOs: `limit_id`, `claim {value, unit, uncertainty?}`, `constraints` (assumption id to a list holding the chosen option id), `parameters` (name to `{value, unit}`). An assumption left "not specified" is omitted from `constraints`. A parameter left empty is omitted from `parameters`: the app never invents a missing quantity.
- R-05 The result panel shows: the verdict with its text label; `bounds` and `margin` when present; the `wording` text exactly as received; the configurations with their `Certain` or `Depends` tag (and `depends_on`); the missing parameters with `NoDomain` or `HasDomain`; the engine version and the limit id. Results are in an `aria-live` region.
- R-06 Verdict styling follows `.kiro/steering/ui-design.md`: each verdict has its own CSS class (`verdict--impossible`, `verdict--conditional`, `verdict--compatible`, `verdict--insufficient-data`) and always its text label. No color encodes a verdict.
- R-07 On HTTP 400 or 422 the app shows the error `code` and `message` in a plain bordered block. For the code `NotApplicable` it also tells the user to choose a reference area. A network failure or a body without the error shape is shown with the code `NetworkError` or `UnexpectedResponse`.
- R-08 Development: `ng serve` uses a proxy that sends `/v1` and `/healthz` to `http://127.0.0.1:3000`. The service uses relative URLs.
- R-09 All user-visible strings live in one file (`src/app/strings.ts`).
- R-10 No external font, CDN, remote asset, icon or emoji. The scaffold's logo, favicon and placeholder page are removed.
- R-11 Tests: HTTP service tests with a mocked backend (`HttpTestingController`); component tests proving each of the four verdicts renders its text label and its own class; form tests for adaptation to the limit and the request body. The runner is Vitest with jsdom, headless, no browser needed.
- R-12 README has a section "Run the web app" with the exact commands.

## Known limits

- `GET /v1/limits` does not expose the dimension expected for the claim or for `Plain` parameters. The UI cannot offer a unit list from the API. It pre-fills `dimensionless` for the claim and for `Plain` parameters, `K` for temperature parameters, and the user can edit the text.
- The API accepts several option ids per assumption; the UI offers one selector per assumption, so it sends one option or none.
- `bounds` values are in SI base units; the UI shows them as received, without a unit.
