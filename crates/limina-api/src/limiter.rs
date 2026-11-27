//! Per-address rate limit of the routes that call a language model
//! (`/v1/analyze`, `/v1/limits/author`, `/v1/transcribe`) for a server open
//! to the public. In memory only; the deterministic routes are never limited.
//!
//! Configuration, from the environment:
//! - `LLM_RATE_LIMIT_PER_HOUR`: requests per address per hour, default 15;
//!   `0` turns the limit off; anything that is not a number gives the default.
//! - `TRUST_PROXY=true`: the address is read from `CF-Connecting-IP`, then
//!   from the first `X-Forwarded-For` entry; otherwise from the connection.
//!   Turn it on only behind a proxy that sets those headers (Cloudflare).
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;
use crate::AppState;

/// Requests per address per hour when `LLM_RATE_LIMIT_PER_HOUR` is not set.
pub const DEFAULT_PER_HOUR: usize = 15;
/// The window of the limit.
pub const WINDOW: Duration = Duration::from_secs(3600);
/// Addresses followed at once; past it the least recently seen is forgotten.
pub const MAX_TRACKED: usize = 10_000;

/// A sliding-window limit per client address.
#[derive(Debug)]
pub struct IpRateLimiter {
    limit: usize,
    window: Duration,
    trust_proxy: bool,
    hits: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl IpRateLimiter {
    /// A limiter of `limit` requests per `window` for each address.
    pub fn new(limit: usize, window: Duration, trust_proxy: bool) -> IpRateLimiter {
        IpRateLimiter {
            limit,
            window,
            trust_proxy,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// The limiter the environment asks for; `None` when it is turned off (`0`).
    pub fn from_env() -> Option<IpRateLimiter> {
        let limit = parse_limit(std::env::var("LLM_RATE_LIMIT_PER_HOUR").ok().as_deref());
        let trust = parse_flag(std::env::var("TRUST_PROXY").ok().as_deref());
        (limit > 0).then(|| IpRateLimiter::new(limit, WINDOW, trust))
    }

    /// Whether the address is read from the proxy headers.
    pub fn trusts_proxy(&self) -> bool {
        self.trust_proxy
    }

    /// Requests allowed per window.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Counts one request of `key`: `Ok`, or the time until a slot frees up.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<(), Duration> {
        let mut hits = self.hits.lock().unwrap_or_else(|p| p.into_inner());
        if !hits.contains_key(key) && hits.len() >= MAX_TRACKED {
            Self::make_room(&mut hits, now, self.window);
        }
        let window = hits.entry(key.to_string()).or_default();
        while window
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            window.pop_front();
        }
        if window.len() >= self.limit {
            let oldest = window.front().copied().unwrap_or(now);
            return Err(self.window.saturating_sub(now.duration_since(oldest)));
        }
        window.push_back(now);
        Ok(())
    }

    /// Drops addresses whose window is empty, and if none is, the one seen longest ago.
    fn make_room(hits: &mut HashMap<String, VecDeque<Instant>>, now: Instant, window: Duration) {
        hits.retain(|_, w| w.back().is_some_and(|t| now.duration_since(*t) < window));
        if hits.len() >= MAX_TRACKED {
            let stalest = hits
                .iter()
                .min_by_key(|(_, w)| w.back().copied())
                .map(|(k, _)| k.clone());
            if let Some(k) = stalest {
                hits.remove(&k);
            }
        }
    }

    /// The key of the client: an address read from the proxy headers when they
    /// are trusted, else the peer of the connection, else "unknown".
    pub fn client_key(&self, headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
        if self.trust_proxy {
            if let Some(ip) = forwarded_ip(headers) {
                return key_of(ip);
            }
        }
        peer.map_or_else(|| "unknown".to_string(), |p| key_of(p.ip()))
    }
}

/// `LLM_RATE_LIMIT_PER_HOUR`: a number (0 turns the limit off), else the default.
pub fn parse_limit(value: Option<&str>) -> usize {
    value
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_PER_HOUR)
}

/// `TRUST_PROXY`: true only for "true" (any case) or "1".
pub fn parse_flag(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        v.eq_ignore_ascii_case("true") || v == "1"
    })
}

fn header_ip(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    headers
        .get(name)?
        .to_str()
        .ok()?
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

/// `CF-Connecting-IP`, else the first `X-Forwarded-For` entry; only a valid address counts.
pub fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    header_ip(headers, "cf-connecting-ip").or_else(|| header_ip(headers, "x-forwarded-for"))
}

/// An address as a key; an IPv6 address by its /64, which one subscriber controls.
fn key_of(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
    }
}

/// The middleware of the routes that call a language model.
pub async fn limit_llm_routes(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(limiter) = state.rate_limiter() else {
        return next.run(request).await;
    };
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0);
    let key = limiter.client_key(request.headers(), peer);
    match limiter.check(&key) {
        Ok(()) => next.run(request).await,
        Err(wait) => {
            let secs = wait.as_secs().max(1);
            let mut response = ApiError::TooManyRequests {
                retry_after_secs: secs,
            }
            .into_response();
            if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, v);
            }
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_str(v).expect("header"));
        }
        h
    }

    #[test]
    fn the_sixteenth_request_of_an_hour_waits_for_the_first_to_leave() {
        let l = IpRateLimiter::new(15, WINDOW, false);
        let t0 = Instant::now();
        for i in 0..15 {
            assert!(l.check_at("a", t0 + Duration::from_secs(i)).is_ok());
        }
        let wait = l
            .check_at("a", t0 + Duration::from_secs(100))
            .expect_err("limited");
        assert_eq!(wait, Duration::from_secs(3600 - 100));
        // Another address is not affected, and the window slides.
        assert!(l.check_at("b", t0 + Duration::from_secs(100)).is_ok());
        assert!(l.check_at("a", t0 + WINDOW).is_ok());
    }

    #[test]
    fn the_environment_reads_the_limit_and_the_proxy_flag() {
        assert_eq!(parse_limit(None), 15);
        assert_eq!(parse_limit(Some("40")), 40);
        assert_eq!(parse_limit(Some(" 7 ")), 7);
        assert_eq!(parse_limit(Some("0")), 0);
        assert_eq!(parse_limit(Some("many")), 15);
        assert_eq!(parse_limit(Some("-3")), 15);
        assert!(parse_flag(Some("true")) && parse_flag(Some("TRUE")) && parse_flag(Some("1")));
        assert!(!parse_flag(None) && !parse_flag(Some("false")) && !parse_flag(Some("yes")));
    }

    #[test]
    fn the_address_comes_from_cloudflare_first_and_only_when_the_proxy_is_trusted() {
        let both = headers(&[
            ("cf-connecting-ip", "203.0.113.7"),
            ("x-forwarded-for", "198.51.100.9, 10.0.0.1"),
        ]);
        let xff = headers(&[("x-forwarded-for", "198.51.100.9, 10.0.0.1")]);
        let peer = Some("192.0.2.1:5000".parse().expect("addr"));
        let trusting = IpRateLimiter::new(1, WINDOW, true);
        assert_eq!(trusting.client_key(&both, peer), "203.0.113.7");
        assert_eq!(trusting.client_key(&xff, peer), "198.51.100.9");
        assert_eq!(trusting.client_key(&HeaderMap::new(), peer), "192.0.2.1");
        assert_eq!(trusting.client_key(&HeaderMap::new(), None), "unknown");
        // A header that is not an address is ignored, not used as a key.
        let junk = headers(&[
            ("cf-connecting-ip", "not-an-ip"),
            ("x-forwarded-for", "also junk"),
        ]);
        assert_eq!(trusting.client_key(&junk, peer), "192.0.2.1");
        let distrusting = IpRateLimiter::new(1, WINDOW, false);
        assert_eq!(distrusting.client_key(&both, peer), "192.0.2.1");
    }

    #[test]
    fn an_ipv6_client_is_counted_by_its_slash_64() {
        let l = IpRateLimiter::new(1, WINDOW, true);
        let a = headers(&[("cf-connecting-ip", "2001:db8:1:2:aaaa::1")]);
        let b = headers(&[("cf-connecting-ip", "2001:db8:1:2:bbbb::9")]);
        let c = headers(&[("cf-connecting-ip", "2001:db8:1:3::1")]);
        assert_eq!(l.client_key(&a, None), l.client_key(&b, None));
        assert_ne!(l.client_key(&a, None), l.client_key(&c, None));
    }

    #[test]
    fn the_table_of_addresses_never_outgrows_its_bound() {
        let l = IpRateLimiter::new(1, WINDOW, false);
        let t0 = Instant::now();
        for i in 0..MAX_TRACKED + 50 {
            assert!(l
                .check_at(
                    &format!("10.{}.{}.{}", i / 65536, (i / 256) % 256, i % 256),
                    t0
                )
                .is_ok());
        }
        assert!(l.hits.lock().expect("lock").len() <= MAX_TRACKED);
    }
}
