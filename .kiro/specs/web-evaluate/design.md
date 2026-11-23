# Design: web-evaluate

Status: v1, frozen. Based on `requirements.md` v1.

## Angular version decision

The current stable Angular (22.2.x) requires Node `^22.22.3 || ^24.15.0 || >=26`. This machine runs Node 20.19.2 and the user will not upgrade it. The most recent major whose CLI accepts Node 20.19.2 is Angular 21 (`@angular/cli@21` engines: `^20.19.0 || ^22.12.0 || >=24.0.0`). The app therefore uses Angular 21.2.x, one major behind current stable. This is a deliberate deviation from "current stable". `angular-eslint` is pinned to 21.4.0 to match.

Pinned versions (exact, `web/package.json`, `save-exact=true` in `web/.npmrc`): @angular/{common,compiler,core,forms,platform-browser,router} 21.2.26; @angular/{build,cli} 21.2.30; @angular/compiler-cli 21.2.26; rxjs 7.8.2; tslib 2.9.1; typescript 5.9.3; vitest 4.1.12; jsdom 28.1.0; eslint 10.15.0; angular-eslint 21.4.0; typescript-eslint 8.59.2; @eslint/js 10.0.1; prettier 3.10.1.

## Commands (run in `web/`)

- Lint: `npm run lint` (`ng lint`, angular-eslint).
- Test: `npm test` (`ng test --no-watch`, Vitest + jsdom, headless).
- Build: `npm run build`.
- Serve: `npm start` (proxy to 127.0.0.1:3000), URL `http://localhost:4200`.

## Files (`web/src/app/`)

- `strings.ts`: every user-visible string.
- `api.models.ts`: TypeScript types mirroring `crates/limina-api/src/dto.rs`.
- `api.service.ts`: `getLimits()` and `evaluate(request)`; HTTP errors become an `ApiFailure {status, code, message}`.
- `result-panel.ts`: verdict, bounds, wording, configurations, missing parameters, engine version, or the error block.
- `limits-list.ts`: the "Limits" section.
- `evaluate-form.ts`: the "Evaluate" page: form state in signals, builds the request, shows the result panel.
- `app.ts`: page shell. `src/styles.css`: global styles. `proxy.conf.json` in `web/`.

## Verdict styles (ui-design.md)

- `verdict--impossible`: solid black background, white uppercase label.
- `verdict--conditional`: dashed border.
- `verdict--compatible`: thin solid border, white background.
- `verdict--insufficient-data`: dotted grey border.

## Error handling

`ApiService` reads `{error: {code, message}}` from the body of a 4xx response. Status 0 gives `NetworkError`; any other shape gives `UnexpectedResponse`. The result panel shows `code` and `message` in a bordered block; `NotApplicable` adds the "choose a reference area" sentence.

## Product-rule check

The UI sends the user's inputs and displays the API response. It never derives, changes or softens a verdict, and shows `wording` as received.
