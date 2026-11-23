//! Settings from the process environment, else from a `.env` file.

use std::collections::BTreeMap;

use limina_extract::ExtractConfig;

/// Parses `KEY=VALUE` lines: blank lines and `#` comments are skipped, an
/// optional `export ` prefix is removed, and one pair of surrounding quotes
/// is trimmed from the value.
pub fn parse_dotenv(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            let value = value.trim();
            let value = ['"', '\'']
                .iter()
                .find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
                .unwrap_or(value);
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

/// The extraction settings: each variable from `process` first, else from
/// the `.env` text when one is given.
pub fn load_config(
    dotenv: Option<&str>,
    process: impl Fn(&str) -> Option<String>,
) -> ExtractConfig {
    let file = dotenv.map(parse_dotenv).unwrap_or_default();
    ExtractConfig::from_lookup(|k| {
        process(k)
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file.get(k).cloned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotenv_lines() {
        let vars = parse_dotenv(
            "# comment\n\nNVIDIA_API_KEY=\"abc\"\nexport LIMINA_LLM_MODEL = m\r\nBAD\n=x\nQ='v'\n",
        );
        assert_eq!(vars.get("NVIDIA_API_KEY").map(String::as_str), Some("abc"));
        assert_eq!(vars.get("LIMINA_LLM_MODEL").map(String::as_str), Some("m"));
        assert_eq!(vars.get("Q").map(String::as_str), Some("v"));
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn the_process_environment_wins_over_the_file() {
        let config = load_config(Some("LIMINA_LLM_MODEL=from-file\nNVIDIA_API_KEY=k"), |k| {
            (k == "LIMINA_LLM_MODEL").then(|| "from-env".to_string())
        });
        assert_eq!(config.model(), "from-env");
        assert!(config.has_key());
        assert!(!load_config(None, |_| None).has_key());
    }
}
