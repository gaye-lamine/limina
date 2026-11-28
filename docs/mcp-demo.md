# MCP demo

## What was and was not done

**Kiro could not be connected to this server in the session that built it.** The workspace registration file `.kiro/settings/mcp.json` could not be written: the write was refused by a permission rule on `.kiro/settings/` (the file tool answered "Blocked by a permission rule"). Without that file Kiro has no `limina` server: asking the session for the tool `limina::list_limits` returned "Use one exact resource ID from the current load_resource catalogue". So **none of the results below is a call made from Kiro.**

What the results below are: a real session with the built `limina-mcp` binary, spoken to over stdio in JSON-RPC (the protocol Kiro uses), against a local Limina API (`http://127.0.0.1:3000`, started once, no language-model key, so extraction is off). Requests and responses are pasted exactly as received, one JSON message per line. The configuration to register the server is in the README, section "MCP server".

Session: protocol version `2025-06-18`, client name `raw-stdio-capture`, server `limina-mcp` (SDK: `rmcp` 3.12.0, the official Rust SDK).

## Raw tool list (`tools/list`)

Request:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

Response, as received:

```json
{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"analyze_text","description":"Read the numeric performance claims out of a paragraph (at most 5000 characters) with the language model behind the Limina API, then check each one against the library. Needs extraction on the Limina server; when it is not available this tool says so explicitly.","inputSchema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"text":{"description":"The text to read the claims from, at most 5000 characters.","type":"string"}},"required":["text"],"type":"object"}},{"name":"evaluate_claim","description":"Check a claimed value against a limit with the Limina deterministic engine. Returns the engine's own answer unchanged: verdict (IMPOSSIBLE, CONDITIONAL, COMPATIBLE or INSUFFICIENT_DATA), wording, bounds, margin, configurations and missing parameters. This tool computes nothing itself. constraints: assumption id to the option ids the text allows. parameters: name to value and unit, temperatures in kelvin.","inputSchema":{"$defs":{"ParameterInput":{"description":"A parameter value with its unit.","properties":{"unit":{"description":"A unit symbol, for example `K`, `m`, `W` or `dimensionless`.","type":"string"},"value":{"description":"The value, in `unit`.","format":"double","type":"number"}},"required":["value","unit"],"type":"object"}},"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"constraints":{"additionalProperties":{"items":{"type":"string"},"type":"array"},"default":{},"description":"Assumption id to the option ids the text allows, for example\n`{\"conversion_process\": [\"heat_engine_cycle\"]}`. An assumption the text\ndoes not state is left out.","type":"object"},"limit_id":{"description":"The id of a limit, for example `carnot`.","type":"string"},"parameters":{"additionalProperties":{"$ref":"#/$defs/ParameterInput"},"default":{},"description":"Parameter name to its value and unit, for example\n`{\"Th\": {\"value\": 600, \"unit\": \"K\"}}`. Temperatures are in kelvin.","type":"object"},"uncertainty":{"default":null,"description":"The uncertainty of the claim, in the same unit; absent means none.","format":"double","type":["number","null"]},"unit":{"description":"A unit symbol, for example `dimensionless`, `W` or `m/s`.","type":"string"},"value":{"description":"The claimed value, in `unit`.","format":"double","type":"number"}},"required":["limit_id","value","unit"],"type":"object"}},{"name":"explain_limit","description":"Describe one limit in full: formula, statement, assumptions with their options, parameters with units, validity domain, limitations and references. Takes an id from list_limits.","inputSchema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"id":{"description":"The id of a limit, as listed by `list_limits`, for example `carnot`.","type":"string"}},"required":["id"],"type":"object"}},{"name":"list_limits","description":"List the physical limits in the Limina library: id, title, direction (Max, Min or Equal), claim unit, formula and a one-sentence statement. Call it first to find a limit id.","inputSchema":{"properties":{},"type":"object"}},{"name":"solve_threshold","description":"Find the value of one free parameter at which a limit's bound equals the claim, with the engine's verdict just below and just above it. Answers questions such as: how hot must the hot reservoir be for this efficiency to be possible? The search is done by the Limina engine, not by this tool.","inputSchema":{"$defs":{"ClaimInput":{"description":"A claimed value.","properties":{"uncertainty":{"description":"The uncertainty of the claim, in the same unit; absent means none.","format":"double","type":["number","null"]},"unit":{"description":"A unit symbol, for example `dimensionless`, `W` or `m/s`.","type":"string"},"value":{"description":"The claimed value, in `unit`.","format":"double","type":"number"}},"required":["value","unit"],"type":"object"},"ParameterInput":{"description":"A parameter value with its unit.","properties":{"unit":{"description":"A unit symbol, for example `K`, `m`, `W` or `dimensionless`.","type":"string"},"value":{"description":"The value, in `unit`.","format":"double","type":"number"}},"required":["value","unit"],"type":"object"}},"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"claim":{"$ref":"#/$defs/ClaimInput","description":"The claim to match."},"constraints":{"additionalProperties":{"items":{"type":"string"},"type":"array"},"default":{},"description":"Assumption id to the option ids the text allows.","type":"object"},"free_parameter":{"description":"The parameter to solve for, for example `Th`.","type":"string"},"limit_id":{"description":"The id of a limit, for example `carnot`.","type":"string"},"parameters":{"additionalProperties":{"$ref":"#/$defs/ParameterInput"},"default":{},"description":"Every parameter other than the free one, with its value and unit.","type":"object"}},"required":["limit_id","claim","free_parameter"],"type":"object"}}]}}
```

## One raw result: `evaluate_claim`

Request:

```json
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"evaluate_claim","arguments":{"limit_id":"carnot","value":0.9,"unit":"dimensionless","constraints":{"cycle_model":["ideal_reversible_two_reservoirs"],"conversion_process":["heat_engine_cycle"]},"parameters":{"Th":{"value":600,"unit":"K"},"Tc":{"value":300,"unit":"K"}}}}}
```

Response, as received (the `text` field holds the API's JSON answer, passed on unchanged; its keys are in alphabetical order because the tool re-serialises the JSON it received):

```json
{"jsonrpc":"2.0","id":5,"result":{"content":[{"type":"text","text":"{\n  \"bounds\": {\n    \"permissive\": {\n      \"b_max\": 0.5,\n      \"b_min\": 0.5,\n      \"margin\": {\n        \"high\": 1.8,\n        \"low\": 1.8\n      }\n    },\n    \"restrictive\": {\n      \"b_max\": 0.5,\n      \"b_min\": 0.5,\n      \"margin\": {\n        \"high\": 1.8,\n        \"low\": 1.8\n      }\n    }\n  },\n  \"configurations\": [],\n  \"engine_version\": \"0.1.0\",\n  \"limit_id\": \"carnot\",\n  \"missing_parameters\": [],\n  \"verdict\": \"IMPOSSIBLE\",\n  \"wording\": \"inconsistent with limit carnot under assumptions conversion process = a heat engine cycle between two reservoirs; cycle model = ideal reversible cycle between two reservoirs\"\n}"}],"isError":false}}
```

The verdict is `IMPOSSIBLE` because the engine computes the Carnot bound 1 - 300/600 = 0.5 and the claim 0.9 is above it, with the text stating a heat engine cycle. The MCP server computes none of it.

## The other tools, same session

| Tool | Arguments | Result | Start of the text |
| --- | --- | --- | --- |
| `list_limits` | `{}` | ok | { "count": 35, "limits": [ { "claim_unit": "dimensionless", "classification": "theoretical_bound", "direction": "Max", "formula": "Cp <= 16/27", "id": ... |
| `explain_limit` | `{"id":"carnot"}` | ok | { "assumption_labels": { "conversion_process": "conversion process", "cycle_model": "cycle model" }, "assumptions": [ { "id": "cycle_model", "options" ... |
| `solve_threshold` | `{"limit_id":"carnot","claim":{"value":0.9,"unit":"dimensionless"},"free_parameter":"Th","constraints":{"cycle_model":["ideal_reversible_two_reservoirs"],"conversion_process":["heat_engine_cycle"]},"parameters":{"Tc":{"value":300,"unit":"K"}}}` | error result | The Limina API answered HTTP 422 (NoThresholdInRange): the bound does not reach the claim for Th in [400, 1000] |
| `solve_threshold` | `{"limit_id":"carnot","claim":{"value":0.6,"unit":"dimensionless"},"free_parameter":"Th","constraints":{"cycle_model":["ideal_reversible_two_reservoirs"],"conversion_process":["heat_engine_cycle"]},"parameters":{"Tc":{"value":300,"unit":"K"}}}` | ok | { "above": 750.00075, "below": 749.99925, "direction": "at least", "free_parameter": "Th", "limit_id": "carnot", "monotonicity": "Increasing", "range" ... |
| `analyze_text` | `{"text":"The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency."}` | error result | Extraction is unavailable on this Limina server: no language-model key is configured there, so a text cannot be read into claims. The deterministic to ... |

Notes on this table:

- `solve_threshold` was called twice. The first call (claim 0.9) is an error result because the API answered HTTP 422 `NoThresholdInRange`: the claim needs a hot reservoir at 3000 K, outside the search range 400 to 1000 K. The error is passed on as is. The second call (claim 0.6) succeeds and finds 750 K.
- `analyze_text` returns the explicit message about extraction because the local API has no key. Against a server with a key it passes the analysis on.
- `list_limits` and `explain_limit` read `GET /v1/limits`; the other three tools call `POST /v1/evaluate`, `/v1/solve` and `/v1/analyze`.

## Check it yourself

```
cargo test -p limina-mcp
```

The tests start the real binary against a mock HTTP server. They check the five tool names and schemas, that each request body reaches the API unchanged, that a mock answer of COMPATIBLE for a claim far above the bound comes back as COMPATIBLE (the tool computes no verdict), that an unreachable API is reported, and that a key placed in the environment of the process never appears in any request.
