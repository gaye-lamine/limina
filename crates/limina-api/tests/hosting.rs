//! A server open to the public: the per-address limit on the routes that call
//! a language model, and the web app served next to the API.
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use limina_api::limiter::IpRateLimiter;
use limina_api::static_files::with_static_dir;
use limina_api::{router_with, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

const HOUR: Duration = Duration::from_secs(3600);

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut b = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let request = b
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("infallible");
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, headers, bytes)
}

fn json_of(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("json")
}

fn author_body() -> Option<Value> {
    Some(json!({"text": "Erasing one bit at temperature T dissipates at least kB T ln 2."}))
}

#[tokio::test]
async fn the_sixteenth_model_request_of_an_address_is_429_with_a_wait() {
    // No key is configured: an unlimited request answers 503, a limited one 429.
    let state = AppState::default().with_rate_limiter(IpRateLimiter::new(15, HOUR, true));
    let app = router_with(state);
    let ip = [("cf-connecting-ip", "203.0.113.7")];
    for i in 0..15 {
        let (status, _, _) = send(&app, "POST", "/v1/limits/author", &ip, author_body()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "request {i}");
    }
    let (status, headers, body) = send(&app, "POST", "/v1/limits/author", &ip, author_body()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json_of(&body)["error"]["code"], "RateLimited");
    assert!(json_of(&body)["error"]["message"]
        .as_str()
        .expect("message")
        .contains("minutes"));
    let wait: u64 = headers["retry-after"]
        .to_str()
        .expect("header")
        .parse()
        .expect("seconds");
    assert!((1..=3600).contains(&wait), "{wait}");
    // Another address still gets through.
    let other = [("cf-connecting-ip", "203.0.113.8")];
    let (status, _, _) = send(&app, "POST", "/v1/limits/author", &other, author_body()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn the_three_model_routes_share_one_budget_and_the_deterministic_routes_have_none() {
    let state = AppState::default().with_rate_limiter(IpRateLimiter::new(3, HOUR, true));
    let app = router_with(state);
    let ip = [("x-forwarded-for", "198.51.100.4, 10.0.0.2")];
    let analyze = Some(json!({"text": "A heat engine reaches 60%."}));
    assert_eq!(
        send(&app, "POST", "/v1/analyze", &ip, analyze.clone())
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(&app, "POST", "/v1/limits/author", &ip, author_body())
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // Whatever the handler answers to this body, the request took one slot of the budget.
    assert_ne!(
        send(
            &app,
            "POST",
            "/v1/transcribe",
            &ip,
            Some(json!({"mime": "image/png", "data": ""}))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    for path in ["/v1/analyze", "/v1/limits/author", "/v1/transcribe"] {
        let (status, _, _) = send(&app, "POST", path, &ip, author_body()).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{path}");
    }
    // The deterministic routes answer as many times as asked from the same address.
    for _ in 0..40 {
        assert_eq!(
            send(&app, "GET", "/v1/limits", &ip, None).await.0,
            StatusCode::OK
        );
        let carnot = json!({
            "limit_id": "carnot",
            "claim": {"value": 0.4, "unit": "dimensionless"},
            "constraints": {"conversion_process": ["heat_engine_cycle"]},
            "parameters": {"Th": {"value": 600.0, "unit": "K"}, "Tc": {"value": 300.0, "unit": "K"}}
        });
        assert_eq!(
            send(&app, "POST", "/v1/evaluate", &ip, Some(carnot))
                .await
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        send(&app, "GET", "/healthz", &ip, None).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn proxy_headers_count_only_when_the_proxy_is_trusted() {
    let state = AppState::default().with_rate_limiter(IpRateLimiter::new(1, HOUR, false));
    let app = router_with(state);
    // Without trust, every request comes from the same (unknown) connection, whatever it claims.
    let (first, _, _) = send(
        &app,
        "POST",
        "/v1/limits/author",
        &[("cf-connecting-ip", "203.0.113.1")],
        author_body(),
    )
    .await;
    let (second, _, _) = send(
        &app,
        "POST",
        "/v1/limits/author",
        &[("cf-connecting-ip", "203.0.113.2")],
        author_body(),
    )
    .await;
    assert_eq!(first, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_server_without_a_limiter_is_not_limited() {
    let app = router_with(AppState::default());
    for _ in 0..30 {
        assert_eq!(
            send(&app, "POST", "/v1/limits/author", &[], author_body())
                .await
                .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn site() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "limina-static-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(dir.join("assets")).expect("dir");
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><title>Limina</title>",
    )
    .expect("index");
    std::fs::write(dir.join("main-ABC123.js"), "console.log(1)").expect("js");
    std::fs::write(dir.join("styles.css"), "body{}").expect("css");
    std::fs::write(dir.join("assets").join("logo.svg"), "<svg/>").expect("svg");
    // A file next to the folder, which a path with ".." must never reach.
    std::fs::write(
        dir.parent().expect("parent").join(format!(
            "{}-secret.txt",
            dir.file_name().expect("name").to_string_lossy()
        )),
        "secret",
    )
    .expect("secret");
    dir
}

#[tokio::test]
async fn the_web_app_is_served_with_a_fallback_to_the_page_for_its_routes() {
    let dir = site();
    let app = with_static_dir(router_with(AppState::default()), dir.clone());

    let (status, headers, body) = send(&app, "GET", "/", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "text/html; charset=utf-8");
    assert_eq!(headers["cache-control"], "no-cache");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert!(String::from_utf8_lossy(&body).contains("Limina"));

    // A client-side route gets the same page.
    let (status, _, body) = send(&app, "GET", "/laboratory", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("Limina"));

    let (status, headers, body) = send(&app, "GET", "/main-ABC123.js", &[], None).await;
    assert_eq!(
        (status, body.as_slice()),
        (StatusCode::OK, b"console.log(1)".as_slice())
    );
    assert_eq!(headers["content-type"], "text/javascript; charset=utf-8");
    assert_eq!(
        send(&app, "GET", "/assets/logo.svg", &[], None).await.1["content-type"],
        "image/svg+xml"
    );

    // A missing file is a 404, not the page; HEAD has no body; other methods are refused.
    assert_eq!(
        send(&app, "GET", "/missing.js", &[], None).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _, body) = send(&app, "HEAD", "/styles.css", &[], None).await;
    assert_eq!((status, body.len()), (StatusCode::OK, 0));
    assert_eq!(
        send(&app, "POST", "/laboratory", &[], None).await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );

    // The API is unchanged, and an unknown API route is a JSON 404, never the page.
    assert_eq!(
        send(&app, "GET", "/v1/limits", &[], None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", "/healthz", &[], None).await.0,
        StatusCode::OK
    );
    let (status, headers, body) = send(&app, "GET", "/v1/unknown", &[], None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers["content-type"], "application/json");
    assert_eq!(json_of(&body)["error"]["code"], "NotFound");

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_path_cannot_leave_the_folder() {
    let dir = site();
    let secret = format!(
        "{}-secret.txt",
        dir.file_name().expect("name").to_string_lossy()
    );
    let app = with_static_dir(router_with(AppState::default()), dir.clone());
    for path in [
        format!("/../{secret}"),
        format!("/assets/../../{secret}"),
        "/..%2f..%2fetc/passwd".to_string(),
        "/assets/..\\..\\x".to_string(),
    ] {
        let (status, _, body) = send(&app, "GET", &path, &[], None).await;
        assert!(!String::from_utf8_lossy(&body).contains("secret"), "{path}");
        assert!(
            status == StatusCode::NOT_FOUND || status == StatusCode::OK,
            "{path}: {status}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(dir.parent().expect("parent").join(&secret)).ok();
}
