//! The model chain: fallback on failure and on unusable answers, the
//! circuit breaker, and two-model authoring when chain B falls back to the
//! model of chain A. Mock providers only: no network.
use std::sync::Arc;
use std::time::Duration;

use limina_extract::testing::NamedMock;
use limina_extract::{
    author_two, ChatReply, ChatRequest, ExtractionProvider, FallbackProvider, LawStatus,
    ProviderError, ReplyKind,
};
use serde_json::json;

const ZERO: Option<Duration> = Some(Duration::ZERO);
const LAW: &str =
    "Erasing one bit at absolute temperature T dissipates at least kB T ln 2 of energy.";

fn request() -> ChatRequest {
    ChatRequest {
        system: "s".into(),
        user: "u".into(),
    }
}

fn chain(mocks: &[&Arc<NamedMock>], kind: ReplyKind) -> FallbackProvider {
    let links: Vec<Arc<dyn ExtractionProvider>> = mocks
        .iter()
        .map(|m| Arc::clone(m) as Arc<dyn ExtractionProvider>)
        .collect();
    FallbackProvider::new(links, kind, Duration::from_secs(5))
}

fn mock(name: &str) -> Arc<NamedMock> {
    Arc::new(NamedMock::new(name))
}

#[tokio::test]
async fn the_first_model_fails_then_the_second_answers() {
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push(Err(ProviderError::Timeout));
    b.inner.push_content(r#"{"claims":[]}"#);
    let c = chain(&[&a, &b], ReplyKind::Json);
    let r = c.complete_routed(request()).await.expect("b answers");
    assert_eq!(r.route.model, "b");
    assert_eq!(r.route.skipped, vec!["a (timeout)".to_string()]);
    assert_eq!(r.route.describe(), "model b; skipped: a (timeout)");
    assert_eq!(c.model(), "a");
    assert_eq!(c.models(), ["a", "b"]);
}

#[tokio::test]
async fn http_404_429_and_5xx_fall_through_401_does_not() {
    for status in [404u16, 429, 500, 503] {
        let (a, b) = (mock("a"), mock("b"));
        a.inner.push(Err(ProviderError::Status(status)));
        b.inner.push_content("{}");
        let r = chain(&[&a, &b], ReplyKind::Json)
            .complete_routed(request())
            .await
            .expect("b answers");
        assert_eq!(r.route.model, "b", "HTTP {status}");
    }
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push(Err(ProviderError::Status(401)));
    b.inner.push_content("{}");
    let err = chain(&[&a, &b], ReplyKind::Json)
        .complete_routed(request())
        .await
        .expect_err("401 is not a model failure");
    assert_eq!(err, ProviderError::Status(401));
    assert_eq!(b.inner.calls(), 0);
}

#[tokio::test]
async fn every_model_failing_gives_the_typed_error_of_the_last() {
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push(Err(ProviderError::Timeout));
    b.inner.push(Err(ProviderError::Status(503)));
    let err = chain(&[&a, &b], ReplyKind::Json)
        .complete(request())
        .await
        .expect_err("all fail");
    assert_eq!(err, ProviderError::Status(503));
}

#[tokio::test]
async fn unusable_answers_fall_through() {
    let (a, b, c, d, e) = (mock("a"), mock("b"), mock("c"), mock("d"), mock("e"));
    a.inner.push_content("   ");
    b.inner.push(Ok(ChatReply {
        content: Some(r#"{"claims": ["#.into()),
        finish_reason: Some("length".into()),
    }));
    c.inner.push_content("no json here");
    d.inner.push_content("{ not: valid json }");
    e.inner.push_content("```json\n{\"claims\": []}\n```");
    let r = chain(&[&a, &b, &c, &d, &e], ReplyKind::Json)
        .complete_routed(request())
        .await
        .expect("e answers");
    assert_eq!(r.route.model, "e");
    assert_eq!(
        r.route.skipped,
        [
            "a (empty answer)",
            "b (answer cut at the token limit)",
            "c (no JSON object in the answer)",
            "d (the JSON object does not parse)",
        ]
    );
}

#[tokio::test]
async fn only_unusable_answers_give_an_unusable_error() {
    let a = mock("a");
    a.inner.push_content("prose");
    let err = chain(&[&a], ReplyKind::Json)
        .complete(request())
        .await
        .expect_err("unusable");
    assert_eq!(err, ProviderError::Unusable("no JSON object in the answer"));
}

#[tokio::test]
async fn a_text_chain_accepts_a_sentence() {
    let a = mock("a");
    a.inner.push_content("no law identified");
    let r = chain(&[&a], ReplyKind::Text)
        .complete_routed(request())
        .await
        .expect("a sentence is usable");
    assert_eq!(r.route.model, "a");
}

#[tokio::test]
async fn the_circuit_breaker_skips_a_failed_model() {
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push(Err(ProviderError::Status(503)));
    b.inner.push_content("{}");
    b.inner.push_content("{}");
    let c = chain(&[&a, &b], ReplyKind::Json);
    c.complete(request()).await.expect("first call");
    let r = c.complete_routed(request()).await.expect("second call");
    assert_eq!(r.route.model, "b");
    assert_eq!(
        r.route.skipped,
        vec!["a (cooling down after a failure)".to_string()]
    );
    assert_eq!(a.inner.calls(), 1, "a is not called while cooling down");
}

#[tokio::test]
async fn the_breaker_closes_after_the_cool_down() {
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push(Err(ProviderError::Status(503)));
    a.inner.push_content("{}");
    b.inner.push_content("{}");
    let c = chain(&[&a, &b], ReplyKind::Json).with_cool_down(Duration::ZERO);
    c.complete(request()).await.expect("first call");
    let r = c.complete_routed(request()).await.expect("second call");
    assert_eq!(r.route.model, "a");
}

#[tokio::test]
async fn when_every_model_is_cooling_down_they_are_tried_anyway() {
    let a = mock("a");
    a.inner.push(Err(ProviderError::Status(503)));
    a.inner.push_content("{}");
    let c = chain(&[&a], ReplyKind::Json);
    assert!(c.complete(request()).await.is_err());
    let r = c.complete_routed(request()).await.expect("tried again");
    assert_eq!(r.route.model, "a");
}

fn landauer(formula: &str) -> String {
    json!({
        "id": "landauer", "title": "Landauer limit", "direction": "min", "bounded_dimension": "J",
        "parameters": [{"name": "T", "kind": "absolute_temperature", "units": ["K"], "required": true, "test_range": [1, 1000]}],
        "formula": formula, "validity": ["T > 0"], "epistemic_status": "theorem"
    })
    .to_string()
}

#[tokio::test]
async fn chain_b_falling_back_to_the_model_of_a_never_agrees() {
    // Chain A: x. Chain B: y (down), then x. Both definitions are written
    // by x and are identical, yet the law has one author only.
    let x_for_a = mock("x");
    let y = mock("y");
    let x_for_b = mock("x");
    x_for_a.inner.push_content(&landauer("kB*T*ln2"));
    y.inner.push(Err(ProviderError::Status(503)));
    x_for_b.inner.push_content(&landauer("kB*T*ln2"));
    let a = chain(&[&x_for_a], ReplyKind::Json);
    let b = chain(&[&y, &x_for_b], ReplyKind::Json);
    let law = author_two(&a, &b, LAW, ZERO).await;
    assert_eq!(law.a.model, "x");
    assert_eq!(law.b.model, "x");
    assert_eq!(law.b.skipped, vec!["y (HTTP 503)".to_string()]);
    assert_eq!(law.status, LawStatus::Disputed);
    assert!(law.comparison.is_none());
    assert!(law.agreed_limit().is_none());
}

#[tokio::test]
async fn two_chains_answered_by_two_models_still_agree() {
    let (x, y, z) = (mock("x"), mock("y"), mock("z"));
    x.inner.push_content(&landauer("kB*T*ln2"));
    y.inner.push(Err(ProviderError::Timeout));
    z.inner.push_content(&landauer("ln2*kB*T"));
    let a = chain(&[&x], ReplyKind::Json);
    let b = chain(&[&y, &z], ReplyKind::Json);
    let law = author_two(&a, &b, LAW, ZERO).await;
    assert_eq!((law.a.model.as_str(), law.b.model.as_str()), ("x", "z"));
    assert_eq!(law.status, LawStatus::Agreed);
}

#[tokio::test]
async fn a_checked_chain_falls_through_on_a_schema_mismatch() {
    let (a, b) = (mock("a"), mock("b"));
    a.inner.push_content(r#"{"claims": "not a list"}"#);
    b.inner.push_content(r#"{"claims": [], "missing": []}"#);
    let r = chain(
        &[&a, &b],
        ReplyKind::Checked(limina_extract::extraction_problem),
    )
    .complete_routed(request())
    .await
    .expect("b answers");
    assert_eq!(r.route.model, "b");
    assert_eq!(
        r.route.skipped,
        vec!["a (the answer does not match the schema)".to_string()]
    );
}
