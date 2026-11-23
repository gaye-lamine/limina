//! Live check of the demo document with the key in `.env`. Ignored by
//! default: `cargo test -p limina-cli --test live -- --ignored --nocapture`.
use std::path::PathBuf;

use limina_api::AppState;
use limina_cli::env::load_config;
use limina_cli::run;

#[tokio::test]
#[ignore]
async fn live_check_of_the_demo() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dotenv = std::fs::read_to_string(root.join(".env")).ok();
    let config = load_config(dotenv.as_deref(), |k| std::env::var(k).ok());
    assert!(
        config.has_key(),
        "NVIDIA_API_KEY is not set in the environment or .env"
    );
    let demo = root.join("demo/spec-heat-pump.md");
    let args = vec!["check".to_string(), demo.display().to_string()];
    let out = run(&args, &config, |c| {
        AppState::from_config(c).map_err(|e| e.to_string())
    })
    .await;
    println!("exit code {}\n{}{}", out.code, out.stdout, out.stderr);
    assert_ne!(out.code, 2, "the analysis did not run: {}", out.stderr);
    if let Some(key) = dotenv
        .as_deref()
        .map(limina_cli::env::parse_dotenv)
        .and_then(|v| v.get("NVIDIA_API_KEY").cloned())
    {
        assert!(!out.stdout.contains(&key) && !out.stderr.contains(&key));
    }
}
