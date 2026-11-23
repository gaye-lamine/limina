# Limina

Limina reads a physical performance claim (press release, product page), extracts the numbers, and checks them against a deterministic library of fundamental physical limits (Carnot, Betz, Shockleyâ€“Queisser, etc.). The verdict is always computed by the deterministic engine, never by an LLM.

**Status:** early development. The engine, the DSL, the API with extraction and authoring, and the web app are implemented; the MCP server is not built yet.

## Installation

1. Install Rust with [rustup](https://rustup.rs/).
2. On Windows, also install the Microsoft C++ Build Tools.
3. Run the tests:

```
cargo test --workspace
```

## Run the API

Start the server (listens on `http://127.0.0.1:3000`; set `LIMINA_API_ADDR` to change the address):

```
cargo run -p limina-api
```

Check it is up, and list the limits:

```
curl http://127.0.0.1:3000/healthz
curl http://127.0.0.1:3000/v1/limits
```

Each limit lists its `title`, `formula` and one-sentence `statement` (display text), its `claim_unit`, for each parameter the allowed `units`, and the display labels of its ids (`assumption_labels`, `option_labels`, `parameter_labels`). For Carnot (abridged):

```json
{"id":"carnot","direction":"Max","title":"Carnot limit","formula":"efficiency <= 1 - Tc/Th","statement":"A heat engine working between ...","claim_unit":"dimensionless","assumptions":[...],"parameters":[{"id":"Th","kind":"AbsoluteTemperature","required":true,"units":["K"]},{"id":"Tc","kind":"AbsoluteTemperature","required":true,"units":["K"]}],"assumption_labels":{"cycle_model":"cycle model"},"option_labels":{"ideal_reversible_two_reservoirs":"ideal reversible cycle between two reservoirs"},"parameter_labels":{"Tc":"cold reservoir temperature","Th":"hot reservoir temperature"}}
```

Betz: a power coefficient of 0.70 for an unshrouded rotor (expected verdict: `IMPOSSIBLE`):

```
curl -X POST http://127.0.0.1:3000/v1/evaluate -H "Content-Type: application/json" -d '{"limit_id":"betz","claim":{"value":0.70,"unit":"dimensionless"},"constraints":{"reference_area":["unshrouded_rotor"]},"parameters":{}}'
```

Carnot: an efficiency of 0.60 between 600 K and 300 K (expected verdict: `IMPOSSIBLE`):

```
curl -X POST http://127.0.0.1:3000/v1/evaluate -H "Content-Type: application/json" -d '{"limit_id":"carnot","claim":{"value":0.60,"unit":"dimensionless"},"constraints":{},"parameters":{"Th":{"value":600,"unit":"K"},"Tc":{"value":300,"unit":"K"}}}'
```

These commands use bash quoting. In Windows PowerShell, save the JSON body to a file and send it with `curl.exe -X POST http://127.0.0.1:3000/v1/evaluate -H "Content-Type: application/json" -d "@body.json"`.

The response holds the verdict, the bounds and margin, the engine version and a `wording` field. A claim is reported as "inconsistent with limit X under assumptions Y", never as anything about a person or company. Errors return `{"error":{"code","message"}}` with status 422 (engine refusal, for example `NotApplicable` when Betz is not constrained) or 400 (invalid JSON).

## Run with extraction

`POST /v1/analyze` reads the claims out of a pasted text with a language model, checks every value against the text, then evaluates each claim with the engine, as `POST /v1/evaluate` does. The model only reads numbers; it never decides a verdict. The key is read from the environment only and is never logged, stored or returned.

In PowerShell, set the key for this session only (it is not echoed and not saved in the history), then start the API:

```
$key = Read-Host -Prompt "NVIDIA API key" -AsSecureString
$env:NVIDIA_API_KEY = [System.Net.NetworkCredential]::new("", $key).Password
cargo run -p limina-api
```

The startup line says `extraction enabled (model ...)`. Optional settings: `LIMINA_LLM_MODEL` (default `nvidia/nemotron-3.5-lightning-30b-a3b`), `LIMINA_LLM_BASE_URL` (default `https://integrate.api.nvidia.com/v1`), `LIMINA_LLM_DISABLE_THINKING` (default `true`). `GET /v1/capabilities` returns `{"extraction": true}`; without a key it returns `false` and `POST /v1/analyze` answers 503 `ExtractionUnavailable`. Remove the key from the session with `Remove-Item Env:NVIDIA_API_KEY`.

```
curl.exe -X POST http://127.0.0.1:3000/v1/analyze -H "Content-Type: application/json" -d "@text.json"
```

with `text.json` holding `{"text": "heat engine between 600 K and 300 K at 60% efficiency"}`. Each claim comes back with its evidence and character offsets, its normalization notes, and either the engine result or `needs_input` with a reason. The live test runs only with a key: `cargo test -p limina-api --test analyze_live -- --ignored`. Never put the key in a file of the repository; `.env` files are ignored by git.

## Declarative limits (DSL)

A limit can be written as JSON in `data/limits/` instead of Rust code (crate `limina-dsl`). Every formula and condition is checked for dimensional consistency when the definition is loaded, then the limit runs in the same decision engine through the core `Limit` trait. Definitions start as `"status": "draft"`; `"validated"` counts only with a person in `validated_by`. `betz.json`, `carnot.json` and `cop.json` reproduce the Rust limits, and differential tests check that both give the same bounds, errors and verdicts.

Grammar of formulas (nothing else is accepted; no assignment, no loop, no unit conversion):

```
condition := expr ("<" | "<=" | ">" | ">=") expr
expr      := term (("+" | "-") term)*
term      := unary (("*" | "/") unary)*
unary     := "-" unary | power
power     := primary ("^" integer)?          -- also ^-2 and ^(-2)
primary   := number | name | function "(" expr ("," expr)* ")" | "(" expr ")"
function  := sqrt | ln | exp | abs | min | max
constants := kB (1.380649e-23 J/K), h (6.62607015e-34 J s), c (299792458 m/s),
             e (1.602176634e-19 C), NA (6.02214076e23 1/mol), pi, ln2
```

Rules: `+` and `-` need equal dimensions; `ln` and `exp` need dimensionless arguments; `sqrt` needs even exponents; the formula dimension must equal `bounded_dimension`. Absolute temperature minus absolute temperature is a difference, and a number has no unit, so `T + 273.15` is refused (no silent Celsius). A validity condition must compare absolute temperatures (the core reports `InvalidTemperatureOrder`); an option `condition` that fails makes the option not applicable. Interval evaluation of a formula that repeats a parameter is conservative (wider than the true range), which can never create a false IMPOSSIBLE.

Example (`data/limits/carnot.json`):

```json
{
  "id": "carnot",
  "title": "Carnot limit",
  "direction": "max",
  "bounded_dimension": "dimensionless",
  "parameters": [
    { "name": "Th", "kind": "absolute_temperature", "units": ["K"], "required": true, "test_range": [400, 1000] },
    { "name": "Tc", "kind": "absolute_temperature", "units": ["K"], "required": true, "test_range": [100, 350] }
  ],
  "assumptions": [
    { "id": "cycle_model", "options": [{ "id": "ideal_reversible_two_reservoirs" }] }
  ],
  "formula": "1 - Tc/Th",
  "validity": ["Tc > 0", "Th > Tc"],
  "references": ["TODO(reference): to be supplied by the author"],
  "status": "draft",
  "epistemic_status": "theorem"
}
```

`cargo test -p limina-dsl` runs the checks, including the differential properties (256 cases, 1024 with `CI` set).
## Inverse questions and authored limits
### Inverse questions: `POST /v1/solve`
Gives the value of one parameter at which the bound equals the claim, every other parameter fixed. The search range is the `range` of the request (SI units), else the plausible domain, else the `test_range` of the definition. The bound must be monotone in that parameter on the range; otherwise the answer is 422 `NotMonotone` and no threshold is given (422 `NoSearchRange` without a range). The verdicts just below and just above the threshold come from the engine. Carnot at 60 % with Tc = 300 K (answer: Th at least 750 K):
```
$body = @{ limit_id = "carnot"; claim = @{ value = 0.60; unit = "dimensionless" }; parameters = @{ Tc = @{ value = 300; unit = "K" } }; free_parameter = "Th" } | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3000/v1/solve -ContentType "application/json" -Body $body
```
### Authored limits: `POST /v1/limits/author`
Writes a DSL definition from one sentence (at most 2000 characters) with the language model, then checks it with the DSL. A refused definition goes back to the model with the exact error, at most twice. The code, not the model, sets `"status": "draft"`, the `draft-` id prefix and the reference "to be added", and never sets `validated_by`. The answer lists the definition, the number of attempts, every error seen and the check report. Authored limits live in memory until the API stops. Needs the key (503 `ExtractionUnavailable` without it):
```
$body = @{ text = "Erasing one bit of information at temperature T dissipates at least kB T ln 2 of energy." } | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3000/v1/limits/author -ContentType "application/json" -Body $body | ConvertTo-Json -Depth 10
```
### Draft limits in `GET /v1/limits` and `POST /v1/evaluate`
An authored limit is listed with `"status": "draft"` and evaluated like the others; its sentences say "inconsistent with draft limit X under assumptions Y". With the id returned above (for example `draft-landauer-principle`), a claim of 1e-21 J at 300 K is below the Landauer bound (about 2.87e-21 J):
```
$body = @{ limit_id = "draft-landauer-principle"; claim = @{ value = 1e-21; unit = "J" }; parameters = @{ T = @{ value = 300; unit = "K" } } } | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3000/v1/evaluate -ContentType "application/json" -Body $body
```
The engine guard is relative only by default (`1e-9 * |bound|`, decision D-24), so verdicts work at any scale; a limit may declare its own absolute floor.

## Physics lint
`limina check` reads a text or markdown file, extracts its performance claims and checks each one with the same code as `POST /v1/analyze`, then prints it at its line and column, like a compiler. It needs the key in the environment or in `.env` in the current directory.
```
cargo run -q -p limina-cli -- check demo/spec-heat-pump.md
cargo run -q -p limina-cli -- check demo/spec-heat-pump.md --format json
```
Each claim prints `file:line:col: severity: VERDICT limit`, then the claim and the bound, the engine wording, a line `to settle this, state: ...` when an assumption or a temperature is missing (mode, reference area), and, where the inverse solver answers, a line such as "Not inconsistent with limit carnot only if Th is at least 750 K". Exit code 0: no inconsistency; 1: at least one IMPOSSIBLE; 2: the analysis could not run (no key, network failure, extraction failed), which wins over 1. CONDITIONAL and needs-input claims are warnings. The file is read paragraph by paragraph (paragraphs without a digit are skipped), and an unusable model answer is asked again up to 3 times; a paragraph that still fails is reported at its line and the check goes on. `demo/spec-heat-pump.md` is an illustrative note with four claims. The live test: `cargo test -p limina-cli --test live -- --ignored --nocapture`.

The Kiro hook `.kiro/hooks/physics-lint.json` (PostFileSave, markdown files under `docs/` or `demo/`) asks the agent to run `limina check` on the saved file, show the report, and for each finding propose either to state the missing assumption or to ask the author; it never changes a number to make a claim pass. The steering file `.kiro/steering/physics-claims.md` applies to the same files.

## Run the web app

Needs Node 20.19 or later (the app uses Angular 21; Angular 22 needs a newer Node). Start the API in one terminal:

```
cargo run -p limina-api
```

Start the web app in a second terminal:

```
cd web
npm install
npm start
```

Open http://localhost:4200 and pick a case in "Open a case". Carnot and COP open on a bench with an apparatus (thermometers for Th and Tc, the claim, and for COP the mode); the instrument panel on the right shows the engine verdict, the wording and the gauge. The Library below lists every limit. The dev server forwards `/v1` and `/healthz` to `http://127.0.0.1:3000`, so the API must be running. Web checks: `npm run lint` and `npm test` (headless, no browser needed).

## License

MIT, see [LICENSE](LICENSE).

## Built with Kiro

This project is built with Kiro. The `.kiro` directory contains the steering, specs and hooks.

