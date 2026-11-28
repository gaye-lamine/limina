//! End to end: the real `limina-mcp` binary, spoken to over stdio, against a
//! mock of the Limina API.

mod common;

use common::{route, start, Session};
use serde_json::{json, Value};

fn limits_body() -> Value {
    json!({"limits": [
        {"id": "carnot", "title": "Carnot limit", "direction": "Max", "claim_unit": "dimensionless",
         "formula": "1 - Tc/Th", "statement": "A heat engine cannot beat 1 - Tc/Th.",
         "classification": "theorem", "parameter_labels": {"Th": "hot reservoir"}, "references": ["x"]},
        {"id": "betz", "title": "Betz limit", "direction": "Max", "claim_unit": "dimensionless",
         "formula": "Cp <= 16/27", "statement": "A rotor takes at most 16/27 of the wind power.",
         "assumptions": [{"id": "reference_area"}]}
    ]})
}

fn evaluate_args() -> Value {
    json!({
        "limit_id": "carnot", "value": 0.9, "unit": "dimensionless",
        "constraints": {"cycle_model": ["ideal_reversible_two_reservoirs"]},
        "parameters": {"Th": {"value": 600, "unit": "K"}, "Tc": {"value": 300, "unit": "K"}}
    })
}

#[test]
fn lists_exactly_the_five_tools_with_schemas() {
    let mock = start(vec![]);
    let mut s = Session::start(&mock.url, &[]);
    let response = s.request("tools/list", json!({}));
    let tools = response["result"]["tools"].as_array().expect("tools");
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "analyze_text",
            "evaluate_claim",
            "explain_limit",
            "list_limits",
            "solve_threshold"
        ]
    );
    let evaluate = tools
        .iter()
        .find(|t| t["name"] == "evaluate_claim")
        .expect("tool");
    let required: Vec<&str> = evaluate["inputSchema"]["required"]
        .as_array()
        .expect("required")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for field in ["limit_id", "value", "unit"] {
        assert!(required.contains(&field), "{field} should be required");
    }
    assert!(
        mock.requests().is_empty(),
        "listing tools must not call the API"
    );
}

#[test]
fn list_limits_keeps_a_short_projection() {
    let mock = start(vec![route("GET", "/v1/limits", 200, limits_body())]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call("list_limits", json!({}));
    assert!(!is_error, "{text}");
    let out: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(out["count"], 2);
    let first = &out["limits"][0];
    assert_eq!(first["id"], "carnot");
    assert_eq!(first["formula"], "1 - Tc/Th");
    assert!(
        first.get("parameter_labels").is_none(),
        "labels are left out of the list"
    );
    assert!(first.get("references").is_none());
}

#[test]
fn explain_limit_returns_the_full_entry_or_names_the_known_ids() {
    let mock = start(vec![route("GET", "/v1/limits", 200, limits_body())]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call("explain_limit", json!({"id": "carnot"}));
    assert!(!is_error, "{text}");
    let out: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(out["parameter_labels"]["Th"], "hot reservoir");
    let (is_error, text) = s.call("explain_limit", json!({"id": "nope"}));
    assert!(is_error);
    assert!(text.contains("Unknown limit id \"nope\""), "{text}");
    assert!(text.contains("carnot, betz"), "{text}");
}

#[test]
fn evaluate_claim_forwards_the_request_and_returns_the_api_answer_unchanged() {
    // The mock answers COMPATIBLE for a claim far above the bound. If the tool
    // computed anything itself, the verdict would differ.
    let api_answer = json!({"verdict": "COMPATIBLE", "limit_id": "carnot", "wording": "from the mock",
                            "bounds": null, "configurations": [], "missing_parameters": []});
    let mock = start(vec![route("POST", "/v1/evaluate", 200, api_answer.clone())]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call("evaluate_claim", evaluate_args());
    assert!(!is_error, "{text}");
    assert_eq!(
        serde_json::from_str::<Value>(&text).expect("json"),
        api_answer
    );

    let seen = mock.requests();
    assert_eq!(seen.len(), 1);
    let sent: Value = serde_json::from_str(&seen[0].body).expect("json body");
    assert_eq!(
        sent,
        json!({
            "limit_id": "carnot",
            "claim": {"value": 0.9, "unit": "dimensionless"},
            "constraints": {"cycle_model": ["ideal_reversible_two_reservoirs"]},
            "parameters": {"Th": {"value": 600.0, "unit": "K"}, "Tc": {"value": 300.0, "unit": "K"}}
        })
    );
}

#[test]
fn evaluate_claim_passes_an_api_error_on_as_an_error_result() {
    let body = json!({"error": {"code": "UnknownLimit", "message": "no limit named zz"}});
    let mock = start(vec![route("POST", "/v1/evaluate", 422, body)]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call("evaluate_claim", evaluate_args());
    assert!(is_error);
    assert!(
        text.contains("HTTP 422")
            && text.contains("UnknownLimit")
            && text.contains("no limit named zz"),
        "{text}"
    );
}

#[test]
fn solve_threshold_forwards_to_solve() {
    let api_answer = json!({"limit_id": "carnot", "free_parameter": "Th", "threshold": 3000.0,
                            "direction": "at least", "sentence": "from the mock"});
    let mock = start(vec![route("POST", "/v1/solve", 200, api_answer.clone())]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call(
        "solve_threshold",
        json!({
            "limit_id": "carnot",
            "claim": {"value": 0.9, "unit": "dimensionless", "uncertainty": 0.01},
            "free_parameter": "Th",
            "constraints": {"cycle_model": ["ideal_reversible_two_reservoirs"]},
            "parameters": {"Tc": {"value": 300, "unit": "K"}}
        }),
    );
    assert!(!is_error, "{text}");
    assert_eq!(
        serde_json::from_str::<Value>(&text).expect("json"),
        api_answer
    );
    let sent: Value = serde_json::from_str(&mock.requests()[0].body).expect("json body");
    assert_eq!(sent["free_parameter"], "Th");
    assert_eq!(sent["claim"]["uncertainty"], 0.01);
    assert_eq!(sent["parameters"]["Tc"]["value"], 300.0);
}

#[test]
fn analyze_text_says_plainly_when_extraction_is_unavailable() {
    let body = json!({"error": {"code": "ExtractionUnavailable", "message": "server has no key"}});
    let mock = start(vec![route("POST", "/v1/analyze", 503, body)]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call(
        "analyze_text",
        json!({"text": "A heat engine at 90 % efficiency."}),
    );
    assert!(is_error);
    assert!(
        text.starts_with("Extraction is unavailable on this Limina server"),
        "{text}"
    );
    assert!(text.contains("evaluate_claim"), "{text}");
}

#[test]
fn analyze_text_passes_a_successful_analysis_on() {
    let api_answer = json!({"claims": [{"quantity": "efficiency", "verdict": "IMPOSSIBLE"}]});
    let mock = start(vec![route("POST", "/v1/analyze", 200, api_answer.clone())]);
    let mut s = Session::start(&mock.url, &[]);
    let (is_error, text) = s.call(
        "analyze_text",
        json!({"text": "A heat engine at 90 % efficiency."}),
    );
    assert!(!is_error, "{text}");
    assert_eq!(
        serde_json::from_str::<Value>(&text).expect("json"),
        api_answer
    );
    let sent: Value = serde_json::from_str(&mock.requests()[0].body).expect("json body");
    assert_eq!(sent, json!({"text": "A heat engine at 90 % efficiency."}));
}

#[test]
fn an_unreachable_api_is_reported_not_hidden() {
    // Port 9 (discard) on localhost is closed in the test environment.
    let mut s = Session::start("http://127.0.0.1:9", &[]);
    let (is_error, text) = s.call("list_limits", json!({}));
    assert!(is_error);
    assert!(
        text.contains("cannot reach the Limina API at http://127.0.0.1:9"),
        "{text}"
    );
}

#[test]
fn no_key_and_no_credential_ever_reaches_the_api() {
    let canary = "canary-key-that-must-never-leave-the-process";
    let mock = start(vec![
        route("GET", "/v1/limits", 200, limits_body()),
        route(
            "POST",
            "/v1/evaluate",
            200,
            json!({"verdict": "COMPATIBLE"}),
        ),
        route("POST", "/v1/solve", 200, json!({"threshold": 1})),
        route("POST", "/v1/analyze", 200, json!({"claims": []})),
    ]);
    let mut s = Session::start(
        &mock.url,
        &[
            ("NVIDIA_API_KEY", canary),
            ("OPENAI_API_KEY", canary),
            ("LIMINA_LLM_API_KEY", canary),
        ],
    );
    s.call("list_limits", json!({}));
    s.call("explain_limit", json!({"id": "carnot"}));
    s.call("evaluate_claim", evaluate_args());
    s.call(
        "solve_threshold",
        json!({"limit_id": "carnot", "claim": {"value": 0.9, "unit": "dimensionless"}, "free_parameter": "Th"}),
    );
    s.call("analyze_text", json!({"text": "x 1"}));
    let seen = mock.requests();
    assert!(seen.len() >= 5);
    for r in seen {
        assert!(!r.body.contains(canary), "key leaked in a body");
        for (name, value) in &r.headers {
            assert!(!value.contains(canary), "key leaked in header {name}");
            assert!(
                !["authorization", "x-api-key", "api-key", "cookie"].contains(&name.as_str()),
                "credential header {name} sent"
            );
        }
    }
}
