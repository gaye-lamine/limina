//! Security rules of spec extract-claims on the API side, written before the
//! implementation: capabilities with and without a key, and a sentinel key
//! that never appears in any response, error or log line.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{router_with, startup_message, AppState};
use limina_extract::ExtractConfig;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tower::ServiceExt;

const SENTINEL: &str = "nvapi-SENTINEL-api-0123456789-do-not-leak";

fn config(base: Option<String>, key: bool) -> ExtractConfig {
    ExtractConfig::from_lookup(move |k| match k {
        "NVIDIA_API_KEY" if key => Some(SENTINEL.to_string()),
        "LIMINA_LLM_BASE_URL" => base.clone(),
        _ => None,
    })
}

async fn call(
    state: AppState,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .expect("request");
    let response = router_with(state)
        .oneshot(request)
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// A local stand-in for the provider: answers every request with `status`
/// and a body that echoes the request, Authorization header included, as
/// `message.content` when `status` is 200.
async fn hostile_server(status: u16) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
                let mut request = String::new();
                loop {
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    request.push_str(&String::from_utf8_lossy(&buf[..n]));
                    let complete = request.find("\r\n\r\n").is_some_and(|end| {
                        let length = request[..end]
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length: ")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        request.len() >= end + 4 + length
                    });
                    if complete || n == 0 {
                        break;
                    }
                }
                // A model output in the schema that quotes the key back.
                let content = json!({
                    "claims": [{
                        "limit_id": "carnot", "quantity": request, "value": 60, "unit": "%",
                        "evidence": "60%", "constraints": {}, "parameters": {}
                    }],
                    "missing": [request]
                })
                .to_string();
                let body = if status == 200 {
                    json!({"choices": [{"message": {"content": content, "reasoning_content": request}, "finish_reason": "stop"}]}).to_string()
                } else {
                    json!({"echo": request}).to_string()
                };
                let response = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}/v1")
}

#[tokio::test]
async fn capabilities_without_a_key_say_no_extraction() {
    let state = AppState::from_config(&config(None, false)).expect("state");
    let (status, body) = call(state, "GET", "/v1/capabilities", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&body).expect("json"),
        json!({"extraction": false})
    );
}

#[tokio::test]
async fn capabilities_with_a_key_say_extraction_and_never_show_it() {
    let state = AppState::from_config(&config(None, true)).expect("state");
    let (status, body) = call(state, "GET", "/v1/capabilities", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&body).expect("json"),
        json!({"extraction": true})
    );
    assert!(!body.contains(SENTINEL));
}

#[tokio::test]
async fn analyze_without_a_key_is_503_extraction_unavailable() {
    let state = AppState::from_config(&config(None, false)).expect("state");
    let (status, body) = call(
        state,
        "POST",
        "/v1/analyze",
        Some(json!({"text": "60% at 600 K"})),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["error"]["code"], "ExtractionUnavailable");
}

#[tokio::test]
async fn the_key_never_comes_back_even_when_the_provider_echoes_it() {
    for status in [200, 401, 500] {
        let base = hostile_server(status).await;
        let state = AppState::from_config(&config(Some(base), true)).expect("state");
        let text = "A heat engine between 600 K and 300 K at 60% efficiency.";
        let (code, body) = call(
            state.clone(),
            "POST",
            "/v1/analyze",
            Some(json!({"text": text})),
        )
        .await;
        assert!(!body.contains(SENTINEL), "HTTP {status} -> {code}: {body}");
        assert!(!body.contains("reasoning_content"), "{body}");
        let (_, caps) = call(state.clone(), "GET", "/v1/capabilities", None).await;
        assert!(!caps.contains(SENTINEL));
        assert!(!format!("{state:?}").contains(SENTINEL));
    }
}

#[test]
fn the_startup_log_line_never_shows_the_key() {
    let with = AppState::from_config(&config(None, true)).expect("state");
    let without = AppState::from_config(&config(None, false)).expect("state");
    let addr = "127.0.0.1:3000".parse().expect("addr");
    let a = startup_message(addr, &with);
    let b = startup_message(addr, &without);
    assert!(!a.contains(SENTINEL));
    assert!(a.contains("extraction enabled"));
    assert!(b.contains("extraction disabled"));
}

#[tokio::test]
async fn evaluate_never_calls_a_language_model() {
    let mock = std::sync::Arc::new(limina_extract::testing::MockProvider::new());
    let state = AppState::with_provider(mock.clone());
    let body = json!({
        "limit_id": "carnot",
        "claim": {"value": 0.6, "unit": "dimensionless"},
        "constraints": {},
        "parameters": {"Th": {"value": 600, "unit": "K"}, "Tc": {"value": 300, "unit": "K"}}
    });
    let (status, _) = call(state, "POST", "/v1/evaluate", Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mock.calls(), 0);
}
