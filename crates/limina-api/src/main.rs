//! Starts the Limina API.
//!
//! Address: `LIMINA_API_ADDR` when set; else `0.0.0.0:$PORT` when `PORT` is
//! set (a hosting platform or a container); else `127.0.0.1:3000`.
//! Extraction is enabled when `NVIDIA_API_KEY` is set.
//! `STATIC_DIR`: a folder holding the built web app, served next to the API.
//! `LLM_RATE_LIMIT_PER_HOUR` (default 15, 0 = off) and `TRUST_PROXY=true`
//! limit the routes that call a language model, per client address.
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use limina_api::static_files::{static_dir_from_env, with_static_dir};
use limina_api::{router_with, startup_message, AppState};

const DEFAULT_ADDR: &str = "127.0.0.1:3000";

/// The address to listen on, from the values of `LIMINA_API_ADDR` and `PORT`.
fn listen_address(addr: Option<&str>, port: Option<&str>) -> Result<SocketAddr, String> {
    if let Some(text) = addr.map(str::trim).filter(|t| !t.is_empty()) {
        return text.parse().map_err(|e| format!("LIMINA_API_ADDR: {e}"));
    }
    if let Some(port) = port.map(str::trim).filter(|t| !t.is_empty()) {
        let port: u16 = port.parse().map_err(|e| format!("PORT: {e}"))?;
        return Ok(SocketAddr::from(([0, 0, 0, 0], port)));
    }
    Ok(DEFAULT_ADDR.parse().expect("the default address is valid"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = listen_address(
        std::env::var("LIMINA_API_ADDR").ok().as_deref(),
        std::env::var("PORT").ok().as_deref(),
    )
    .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
    let state = AppState::from_env().map_err(|e| Error::other(e.to_string()))?;
    let mut app = router_with(state.clone());
    if let Some(dir) = static_dir_from_env() {
        println!("serving the web app from {}", dir.display());
        app = with_static_dir(app, dir);
    }
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("{}", startup_message(addr, &state));
    // The connection's address is the client's when no proxy is trusted.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_comes_from_the_explicit_address_then_the_port_then_the_loopback() {
        assert_eq!(
            listen_address(None, None),
            Ok("127.0.0.1:3000".parse().unwrap())
        );
        assert_eq!(
            listen_address(None, Some("8080")),
            Ok("0.0.0.0:8080".parse().unwrap())
        );
        assert_eq!(
            listen_address(Some("127.0.0.1:4000"), Some("8080")),
            Ok("127.0.0.1:4000".parse().unwrap())
        );
        assert_eq!(
            listen_address(Some(" "), Some("")),
            Ok("127.0.0.1:3000".parse().unwrap())
        );
        assert!(listen_address(None, Some("http"))
            .unwrap_err()
            .starts_with("PORT"));
        assert!(listen_address(Some("nowhere"), None)
            .unwrap_err()
            .starts_with("LIMINA_API_ADDR"));
    }
}
