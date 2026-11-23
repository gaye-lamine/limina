//! `limina check` end to end with a mock provider: no network
//! (spec physics-lint, AC 2).
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use limina_api::{registry, AppState};
use limina_cli::env::load_config;
use limina_cli::position::line_col;
use limina_cli::run;
use limina_extract::testing::MockProvider;
use limina_extract::{ExtractConfig, Extractor};
use serde_json::{json, Value};

const SENTINEL: &str = "sk-SENTINEL-must-never-appear-0123456789";

fn demo_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../demo/spec-heat-pump.md")
}

fn config(key: Option<&str>, base_url: Option<&str>) -> ExtractConfig {
    let key = key.map(str::to_string);
    let base = base_url.map(str::to_string);
    load_config(None, move |k| match k {
        "NVIDIA_API_KEY" => key.clone(),
        "LIMINA_LLM_BASE_URL" => base.clone(),
        _ => None,
    })
}

fn mock_state(mock: &Arc<MockProvider>) -> AppState {
    AppState::with_extractor(
        Extractor::new(mock.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    )
}

fn claim_json(limit: &str, value: f64, unit: &str, evidence: &str, extra: Value) -> String {
    let mut claim = json!({
        "limit_id": limit, "quantity": "q", "value": value, "unit": unit,
        "evidence": evidence, "constraints": {}, "parameters": {}
    });
    if let Some(map) = extra.as_object() {
        for (k, v) in map {
            claim[k] = v.clone();
        }
    }
    json!({"claims": [claim], "missing": []}).to_string()
}

fn kelvin(v: f64) -> Value {
    json!({"value": v, "unit": "K", "evidence": format!("{v} K")})
}

/// The four model answers for the four claim paragraphs of the demo.
fn demo_answers(mock: &MockProvider) {
    mock.push_content(&claim_json(
        "carnot",
        60.0,
        "%",
        "thermal efficiency of 60%",
        json!({"parameters": {"Th": kelvin(600.0), "Tc": kelvin(300.0)}}),
    ));
    mock.push_content(&claim_json(
        "cop",
        14.5,
        "dimensionless",
        "COP of 14.5",
        json!({"parameters": {"Th": kelvin(300.0), "Tc": kelvin(280.0)}}),
    ));
    mock.push_content(&claim_json(
        "betz",
        0.45,
        "dimensionless",
        "power coefficient of 0.45",
        json!({"constraints": {"reference_area": "unshrouded_rotor"}}),
    ));
    mock.push_content(&claim_json(
        "betz",
        0.70,
        "dimensionless",
        "power coefficient of 0.70",
        json!({}),
    ));
}

fn args(path: &std::path::Path, json: bool) -> Vec<String> {
    let mut a = vec!["check".to_string(), path.display().to_string()];
    if json {
        a.extend(["--format".to_string(), "json".to_string()]);
    }
    a
}

async fn run_mock(mock: &Arc<MockProvider>, path: &std::path::Path, json: bool) -> run::Outcome {
    let state = mock_state(mock);
    run(&args(path, json), &config(Some(SENTINEL), None), |_| {
        Ok(state)
    })
    .await
}

fn temp_file(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("limina-cli-{}-{name}", std::process::id()));
    std::fs::write(&path, text).expect("temp file");
    path
}

/// Line and column of `needle` in the demo, computed here from bytes.
fn position_in_demo(needle: &str) -> (usize, usize) {
    let text = std::fs::read_to_string(demo_path()).expect("demo");
    let byte = text.find(needle).expect("in demo");
    let before = &text[..byte];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, text[line_start..byte].chars().count() + 1)
}

#[tokio::test]
async fn the_demo_gives_one_of_each_and_exit_code_1() {
    let mock = Arc::new(MockProvider::new());
    demo_answers(&mock);
    let out = run_mock(&mock, &demo_path(), false).await;
    assert_eq!(out.code, 1, "{}{}", out.stdout, out.stderr);
    assert_eq!(mock.calls(), 4, "one call per claim paragraph");
    let text = &out.stdout;
    assert_eq!(
        text.matches(": error: IMPOSSIBLE carnot").count(),
        1,
        "{text}"
    );
    assert_eq!(
        text.matches(": warning: CONDITIONAL cop").count(),
        1,
        "{text}"
    );
    assert_eq!(text.matches(": ok: COMPATIBLE betz").count(), 1, "{text}");
    assert_eq!(
        text.matches(": warning: NEEDS_INPUT betz").count(),
        1,
        "{text}"
    );
    assert!(text.contains("4 claims: 1 impossible, 1 conditional, 1 compatible, 0 insufficient data, 1 need input"), "{text}");
    let (line, col) = position_in_demo("thermal efficiency of 60%");
    assert!(
        text.contains(&format!(
            "spec-heat-pump.md:{line}:{col}: error: IMPOSSIBLE carnot"
        )),
        "{text}"
    );
    assert!(
        text.contains("inconsistent with limit carnot under assumptions"),
        "{text}"
    );
    assert!(text.contains("Th is at least 750 K"), "{text}");
    assert!(
        text.contains("to settle this, state: mode: \"heating\" or \"cooling\""),
        "{text}"
    );
    assert!(
        text.contains("to settle this, state: reference area: "),
        "{text}"
    );
    assert!(
        text.contains("needs input: reference area not stated"),
        "{text}"
    );
    assert!(!text.contains(SENTINEL) && !out.stderr.contains(SENTINEL));
}

#[tokio::test]
async fn json_has_the_analysis_fields_plus_file_line_and_column() {
    let mock = Arc::new(MockProvider::new());
    demo_answers(&mock);
    let out = run_mock(&mock, &demo_path(), true).await;
    assert_eq!(out.code, 1);
    let body: Value = serde_json::from_str(&out.stdout).expect("json");
    assert!(body["missing"].is_array());
    assert!(body["extraction"]["model"].is_string());
    let claims = body["claims"].as_array().expect("claims");
    assert_eq!(claims.len(), 4);
    let verdicts: Vec<Value> = claims
        .iter()
        .map(|c| c["result"]["verdict"].clone())
        .collect();
    assert_eq!(
        verdicts,
        [
            json!("IMPOSSIBLE"),
            json!("CONDITIONAL"),
            json!("COMPATIBLE"),
            Value::Null
        ]
    );
    assert_eq!(claims[3]["status"], "needs_input");
    assert_eq!(claims[3]["code"], "NotApplicable");
    for c in claims {
        for field in [
            "file", "line", "column", "limit_id", "value", "evidence", "status", "flags", "notes",
        ] {
            assert!(c.get(field).is_some(), "{field} in {c}");
        }
    }
    let (line, col) = position_in_demo("COP of 14.5");
    assert_eq!(
        (claims[1]["line"].clone(), claims[1]["column"].clone()),
        (json!(line), json!(col))
    );
    // Evidence offsets are relative to the whole file.
    let text = std::fs::read_to_string(demo_path()).expect("demo");
    let start = claims[1]["evidence"]["start"].as_u64().expect("start") as usize;
    assert_eq!(line_col(&text, start), (line, col));
    let ev: String = text.chars().skip(start).take(11).collect();
    assert_eq!(ev, "COP of 14.5");
    assert!(claims[0]["inverse"]
        .as_str()
        .expect("inverse")
        .contains("at least 750 K"));
    assert!(!out.stdout.contains(SENTINEL));
}

#[tokio::test]
async fn only_compatible_claims_exit_0_and_multi_byte_text_keeps_columns() {
    let text = "Éolienne « Boreal » : with an unshrouded rotor, a power coefficient of 0.45.\n";
    let path = temp_file("ok.md", text);
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&claim_json(
        "betz",
        0.45,
        "dimensionless",
        "power coefficient of 0.45",
        json!({"constraints": {"reference_area": "unshrouded_rotor"}}),
    ));
    let out = run_mock(&mock, &path, false).await;
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    let col = text[..text.find("power").expect("power")].chars().count() + 1;
    assert!(
        out.stdout
            .contains(&format!(":1:{col}: ok: COMPATIBLE betz")),
        "{}",
        out.stdout
    );
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn conditional_alone_is_a_warning_and_exits_0() {
    let path = temp_file(
        "cop.md",
        "Heat pump: COP of 14.5 between 300 K and 280 K.\n",
    );
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&claim_json(
        "cop",
        14.5,
        "dimensionless",
        "COP of 14.5",
        json!({"parameters": {"Th": kelvin(300.0), "Tc": kelvin(280.0)}}),
    ));
    let out = run_mock(&mock, &path, false).await;
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains(": warning: CONDITIONAL cop"));
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn a_failed_extraction_exits_2() {
    let mock = Arc::new(MockProvider::new());
    for _ in 0..3 {
        mock.push_content("this is not JSON");
    }
    // The other three paragraphs still get their answers.
    let rest = [
        claim_json(
            "cop",
            14.5,
            "dimensionless",
            "COP of 14.5",
            json!({"parameters": {"Th": kelvin(300.0), "Tc": kelvin(280.0)}}),
        ),
        claim_json(
            "betz",
            0.45,
            "dimensionless",
            "power coefficient of 0.45",
            json!({"constraints": {"reference_area": "unshrouded_rotor"}}),
        ),
        claim_json(
            "betz",
            0.70,
            "dimensionless",
            "power coefficient of 0.70",
            json!({}),
        ),
    ];
    for answer in &rest {
        mock.push_content(answer);
    }
    let out = run_mock(&mock, &demo_path(), false).await;
    assert_eq!(out.code, 2, "2 wins even without IMPOSSIBLE");
    assert_eq!(
        mock.calls(),
        6,
        "three answers for the first paragraph, one for each other"
    );
    let (line, _) = position_in_demo("The auxiliary heat engine runs");
    assert!(
        out.stdout.contains(&format!(
            "spec-heat-pump.md:{line}:1: error: ANALYSIS_FAILED ExtractionFailed"
        )),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("1 paragraph(s) could not be analysed"));
    assert!(out.stdout.contains(": warning: CONDITIONAL cop"));
    assert!(out.stderr.contains("ExtractionFailed"), "{}", out.stderr);
}

#[tokio::test]
async fn an_unusable_answer_is_asked_again() {
    let path = temp_file(
        "retry.md",
        "With an unshrouded rotor, a power coefficient of 0.45.\n",
    );
    let mock = Arc::new(MockProvider::new());
    mock.push_content("{\"claims\": [");
    mock.push_content(&claim_json(
        "betz",
        0.45,
        "dimensionless",
        "power coefficient of 0.45",
        json!({"constraints": {"reference_area": "unshrouded_rotor"}}),
    ));
    let out = run_mock(&mock, &path, false).await;
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    assert_eq!(mock.calls(), 2);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn no_key_exits_2_without_calling_the_model() {
    let mock = Arc::new(MockProvider::new());
    let state = mock_state(&mock);
    let out = run(&args(&demo_path(), false), &config(None, None), |_| {
        Ok(state)
    })
    .await;
    assert_eq!(out.code, 2);
    assert!(
        out.stderr.contains("NVIDIA_API_KEY is not set"),
        "{}",
        out.stderr
    );
    assert_eq!(mock.calls(), 0);
}

#[tokio::test]
async fn bad_arguments_and_unreadable_files_exit_2() {
    let mock = Arc::new(MockProvider::new());
    let out = run(&["lint".to_string()], &config(Some(SENTINEL), None), |_| {
        Ok(mock_state(&mock))
    })
    .await;
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("usage: limina check"));
    let missing = std::env::temp_dir().join("limina-cli-does-not-exist.md");
    let out = run_mock(&mock, &missing, false).await;
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("cannot read"));
}

#[tokio::test]
async fn a_network_failure_exits_2_and_never_prints_the_key() {
    // A real provider pointed at a closed local port: the connection fails.
    let out = run(
        &args(&demo_path(), false),
        &config(Some(SENTINEL), Some("http://127.0.0.1:9/v1")),
        |c| AppState::from_config(c).map_err(|e| e.to_string()),
    )
    .await;
    assert_eq!(out.code, 2, "{}{}", out.stdout, out.stderr);
    assert!(out.stderr.contains("analysis failed"), "{}", out.stderr);
    assert!(
        out.stderr.contains("ExtractionProviderError"),
        "{}",
        out.stderr
    );
    assert_eq!(
        out.stderr.lines().count(),
        1,
        "a provider failure stops the check after the first paragraph"
    );
    assert!(!out.stdout.contains(SENTINEL) && !out.stderr.contains(SENTINEL));
}
