//! The command itself, with its output collected so tests can read it.

use limina_api::AppState;
use limina_extract::ExtractConfig;

use crate::args::{parse, Format};
use crate::check::check;
use crate::render;

/// No IMPOSSIBLE claim.
pub const EXIT_OK: i32 = 0;
/// At least one IMPOSSIBLE claim.
pub const EXIT_IMPOSSIBLE: i32 = 1;
/// The analysis could not run.
pub const EXIT_FAILED: i32 = 2;

/// Exit code and output of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// 0, 1 or 2.
    pub code: i32,
    /// The report.
    pub stdout: String,
    /// Errors.
    pub stderr: String,
}

fn failed(message: String) -> Outcome {
    Outcome {
        code: EXIT_FAILED,
        stdout: String::new(),
        stderr: format!("limina: {message}\n"),
    }
}

/// Runs `limina <args>`. `make_state` builds the extraction from the
/// configuration; it is called only when the configuration has a key.
pub async fn run(
    args: &[String],
    config: &ExtractConfig,
    make_state: impl FnOnce(&ExtractConfig) -> Result<AppState, String>,
) -> Outcome {
    let command = match parse(args) {
        Ok(c) => c,
        Err(usage) => return failed(usage),
    };
    if !config.has_key() {
        return failed(
            "NVIDIA_API_KEY is not set (neither in the environment nor in .env); \
             the claims cannot be extracted, nothing was checked"
                .to_string(),
        );
    }
    let file = command.path.display().to_string();
    let text = match std::fs::read_to_string(&command.path) {
        Ok(t) => t,
        Err(e) => return failed(format!("cannot read {file}: {e}")),
    };
    let state = match make_state(config) {
        Ok(s) => s,
        Err(e) => return failed(format!("extraction unavailable: {e}")),
    };
    let report = check(&state, &file, &text, config.model()).await;
    let stdout = match command.format {
        Format::Text => render::text(&report),
        Format::Json => render::json(&report),
    };
    let (code, stderr) = if !report.errors.is_empty() {
        let lines: Vec<String> = report
            .errors
            .iter()
            .map(|e| {
                format!(
                    "limina: analysis failed at {}:{}: {}: {}\n",
                    e.file, e.line, e.code, e.message
                )
            })
            .collect();
        (EXIT_FAILED, lines.concat())
    } else if report.impossible() > 0 {
        (EXIT_IMPOSSIBLE, String::new())
    } else {
        (EXIT_OK, String::new())
    };
    Outcome {
        code,
        stdout,
        stderr,
    }
}
