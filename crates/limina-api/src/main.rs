//! Starts the Limina API. Address from `LIMINA_API_ADDR`, default
//! `127.0.0.1:3000`. Extraction is enabled when `NVIDIA_API_KEY` is set.

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use limina_api::{router_with, startup_message, AppState};

const DEFAULT_ADDR: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let text = std::env::var("LIMINA_API_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let addr: SocketAddr = text
        .parse()
        .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("LIMINA_API_ADDR: {e}")))?;
    let state = AppState::from_env().map_err(|e| Error::other(e.to_string()))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("{}", startup_message(addr, &state));
    axum::serve(listener, router_with(state)).await
}
