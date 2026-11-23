//! Configuration, read only from environment variables.

use std::fmt;

/// Default model id.
pub const DEFAULT_MODEL: &str = "nvidia/nemotron-3.5-lightning-30b-a3b";
/// Default OpenAI-compatible base URL.
pub const DEFAULT_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

/// The provider key. It has no `Display` and no `Serialize`; its `Debug`
/// prints `[redacted]`. Only the provider implementation reads it.
#[derive(Clone)]
pub struct SecretKey(String);

impl SecretKey {
    /// Wraps a key.
    pub fn new(key: String) -> SecretKey {
        SecretKey(key)
    }

    /// The key itself, for the Authorization header only.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// Extraction settings.
///
/// - `NVIDIA_API_KEY`: the key; extraction is unavailable without it.
/// - `LIMINA_LLM_MODEL`: model id, default [`DEFAULT_MODEL`].
/// - `LIMINA_LLM_BASE_URL`: base URL, default [`DEFAULT_BASE_URL`].
/// - `LIMINA_LLM_DISABLE_THINKING`: default true; any of `false`, `0`, `no`
///   (any case) turns it off.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    model: String,
    base_url: String,
    disable_thinking: bool,
    api_key: Option<SecretKey>,
}

impl ExtractConfig {
    /// Reads the settings through `lookup`, so tests never touch the process
    /// environment. Blank values count as absent.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> ExtractConfig {
        let get = |k: &str| {
            lookup(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let disable_thinking = get("LIMINA_LLM_DISABLE_THINKING")
            .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "no"))
            .unwrap_or(true);
        ExtractConfig {
            model: get("LIMINA_LLM_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url: get("LIMINA_LLM_BASE_URL")
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            disable_thinking,
            api_key: get("NVIDIA_API_KEY").map(SecretKey::new),
        }
    }

    /// Reads the settings from the process environment.
    pub fn from_env() -> ExtractConfig {
        ExtractConfig::from_lookup(|k| std::env::var(k).ok())
    }

    /// Model id.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Base URL, without a trailing slash.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Whether `chat_template_kwargs {"enable_thinking": false}` is sent.
    pub fn disable_thinking(&self) -> bool {
        self.disable_thinking
    }

    /// Whether a key is set, so extraction is available.
    pub fn has_key(&self) -> bool {
        self.api_key.is_some()
    }

    pub(crate) fn api_key(&self) -> Option<&SecretKey> {
        self.api_key.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(pairs: &[(&str, &str)]) -> ExtractConfig {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        ExtractConfig::from_lookup(move |k| {
            pairs.iter().find(|(p, _)| p == k).map(|(_, v)| v.clone())
        })
    }

    #[test]
    fn defaults() {
        let c = config(&[]);
        assert_eq!(c.model(), DEFAULT_MODEL);
        assert_eq!(c.base_url(), DEFAULT_BASE_URL);
        assert!(c.disable_thinking());
        assert!(!c.has_key());
    }

    #[test]
    fn overrides_and_trailing_slash() {
        let c = config(&[
            ("LIMINA_LLM_MODEL", "m"),
            ("LIMINA_LLM_BASE_URL", "http://x/v1/"),
            ("LIMINA_LLM_DISABLE_THINKING", "False"),
            ("NVIDIA_API_KEY", "k"),
        ]);
        assert_eq!(c.model(), "m");
        assert_eq!(c.base_url(), "http://x/v1");
        assert!(!c.disable_thinking());
        assert!(c.has_key());
    }

    #[test]
    fn a_blank_key_is_no_key() {
        assert!(!config(&[("NVIDIA_API_KEY", "  ")]).has_key());
    }
}
