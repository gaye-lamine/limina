//! `limina check <file> [--format text|json]`. Settings from the process
//! environment, else from `.env` in the current directory. Exit code 0: no
//! IMPOSSIBLE claim; 1: at least one; 2: the analysis could not run.

use std::io::Write;

use limina_api::AppState;
use limina_cli::env::load_config;
use limina_cli::run;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dotenv = std::fs::read_to_string(".env").ok();
    let config = load_config(dotenv.as_deref(), |k| std::env::var(k).ok());
    let outcome = run(&args, &config, |c| {
        AppState::from_config(c).map_err(|e| e.to_string())
    })
    .await;
    let _ = std::io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = std::io::stderr().write_all(outcome.stderr.as_bytes());
    std::process::exit(outcome.code);
}
