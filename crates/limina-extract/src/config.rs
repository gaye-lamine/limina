//! Configuration, read only from environment variables.

use std::fmt;

/// Default model id.
pub const DEFAULT_MODEL: &str = "nvidia/nemotron-3.5-lightning-30b-a3b";
/// Default OpenAI-compatible base URL.
pub const DEFAULT_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

/// Default second model for two-model law authoring (`LIMINA_LLM_MODEL_B`).
pub const DEFAULT_SECOND_MODEL: &str = "nvidia/nemotron-3-ultra-550b-a55b";

/// Default vision-language model for image input (`LIMINA_LLM_VISION_MODEL`).
pub const DEFAULT_VISION_MODEL: &str = "nvidia/nemotron-3.5-super-vl-120b-a12b";

/// Default chain for extraction, law naming and parameters
/// (`LIMINA_LLM_MODELS`), fastest first.
pub const DEFAULT_MODELS: [&str; 4] = [
    "nvidia/nemotron-3.5-lightning-30b-a3b",
    "nvidia/nemotron-3.5-bolt-e4b",
    "poolside/laguna-xs-2.1",
    "nvidia/nemotron-3-ultra-550b-a55b",
];
/// Default chain of authoring model B (`LIMINA_LLM_MODELS_B`): another
/// family first, so the two authors differ whenever it answers.
pub const DEFAULT_MODELS_B: [&str; 2] = [
    "poolside/laguna-xs-2.1",
    "nvidia/nemotron-3-ultra-550b-a55b",
];
/// Default chain of vision models (`LIMINA_LLM_VISION_MODELS`).
pub const DEFAULT_VISION_MODELS: [&str; 2] = [
    "nvidia/nemotron-3.5-super-vl-120b-a12b",
    "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning",
];
/// Default time allowed to one model, in seconds (`LIMINA_LLM_TIMEOUT_SECS`).
pub const DEFAULT_TIMEOUT_SECS: u64 = 15;
/// Seconds a failed model is skipped (circuit breaker).
pub const COOL_DOWN_SECS: u64 = 60;
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
    second_model: String,
    vision_model: String,
    models: Vec<String>,
    models_b: Vec<String>,
    vision_models: Vec<String>,
    timeout_secs: u64,
}
/// A comma-separated list, trimmed, without blanks or duplicates.
fn list(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !out.iter().any(|o| o == item) {
            out.push(item.to_string());
        }
    }
    out
}
/// `first` (the single-model variable, when set) followed by the chain
/// read from `chain` or `default`, without duplicates.
fn chain(first: Option<String>, chain: Option<String>, default: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = first.into_iter().collect();
    let rest = chain.map_or_else(
        || default.iter().map(|s| s.to_string()).collect(),
        |c| list(&c),
    );
    for m in rest {
        if !out.contains(&m) {
            out.push(m);
        }
    }
    out
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
        let models = chain(
            get("LIMINA_LLM_MODEL"),
            get("LIMINA_LLM_MODELS"),
            &DEFAULT_MODELS,
        );
        let models_b = chain(
            get("LIMINA_LLM_MODEL_B"),
            get("LIMINA_LLM_MODELS_B"),
            &DEFAULT_MODELS_B,
        );
        let vision_models = chain(
            get("LIMINA_LLM_VISION_MODEL"),
            get("LIMINA_LLM_VISION_MODELS"),
            &DEFAULT_VISION_MODELS,
        );
        let timeout_secs = get("LIMINA_LLM_TIMEOUT_SECS")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|s| *s > 0)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        ExtractConfig {
            models,
            models_b,
            vision_models,
            timeout_secs,
            model: get("LIMINA_LLM_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url: get("LIMINA_LLM_BASE_URL")
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            disable_thinking,
            api_key: get("NVIDIA_API_KEY").map(SecretKey::new),
            second_model: get("LIMINA_LLM_MODEL_B")
                .unwrap_or_else(|| DEFAULT_SECOND_MODEL.to_string()),
            vision_model: get("LIMINA_LLM_VISION_MODEL")
                .unwrap_or_else(|| DEFAULT_VISION_MODEL.to_string()),
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

    /// Second model id, for two-model law authoring (`LIMINA_LLM_MODEL_B`).
    pub fn second_model(&self) -> &str {
        &self.second_model
    }

    /// Vision-language model id, for image input (`LIMINA_LLM_VISION_MODEL`).
    pub fn vision_model(&self) -> &str {
        &self.vision_model
    }

    /// Chain for extraction, law naming and parameters: `LIMINA_LLM_MODEL`
    /// first when set, then `LIMINA_LLM_MODELS` or [`DEFAULT_MODELS`].
    pub fn models(&self) -> &[String] {
        &self.models
    }
    /// Chain of authoring model B (`LIMINA_LLM_MODELS_B`).
    pub fn models_b(&self) -> &[String] {
        &self.models_b
    }
    /// Chain of vision models (`LIMINA_LLM_VISION_MODELS`).
    pub fn vision_models(&self) -> &[String] {
        &self.vision_models
    }
    /// Time allowed to one model.
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs)
    }
    /// The same settings with another model id.
    pub fn with_model(&self, model: &str) -> ExtractConfig {
        let mut c = self.clone();
        c.model = model.to_string();
        c
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
    fn chains_default_and_the_single_model_variable_comes_first() {
        let c = config(&[]);
        assert_eq!(c.models(), DEFAULT_MODELS.map(String::from));
        assert_eq!(c.models_b()[0], "poolside/laguna-xs-2.1");
        assert_eq!(c.vision_models().len(), 2);
        assert_eq!(c.timeout(), std::time::Duration::from_secs(15));
        let c = config(&[
            ("LIMINA_LLM_MODEL", "m"),
            ("LIMINA_LLM_MODELS", "a, b ,,a,m"),
            ("LIMINA_LLM_TIMEOUT_SECS", "5"),
        ]);
        assert_eq!(c.models(), ["m", "a", "b"]);
        assert_eq!(c.timeout(), std::time::Duration::from_secs(5));
        let bad = config(&[("LIMINA_LLM_TIMEOUT_SECS", "0")]);
        assert_eq!(bad.timeout(), std::time::Duration::from_secs(15));
    }
    #[test]
    fn a_blank_key_is_no_key() {
        assert!(!config(&[("NVIDIA_API_KEY", "  ")]).has_key());
    }
}
