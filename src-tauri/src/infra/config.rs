use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_base_url: String,
    pub whisper_timeout_secs: u64,
    pub max_audio_duration_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://api.nan.builders/v1".to_string(),
            whisper_timeout_secs: 120,
            max_audio_duration_secs: 300,
        }
    }
}

pub struct ConfigManager {
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.config.api_base_url = url;
        self
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.clone()
    }

    pub fn transcription_endpoint(&self) -> String {
        format!("{}/audio/transcriptions", self.config.api_base_url)
    }

    pub fn translation_endpoint(&self) -> String {
        format!("{}/audio/translations", self.config.api_base_url)
    }

    pub fn chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.config.api_base_url)
    }

    pub fn embeddings_endpoint(&self) -> String {
        format!("{}/embeddings", self.config.api_base_url)
    }

    pub fn rerank_endpoint(&self) -> String {
        format!("{}/rerank", self.config.api_base_url)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.api_base_url, "https://api.nan.builders/v1");
        assert_eq!(cfg.max_audio_duration_secs, 300);
    }

    #[test]
    fn test_config_manager_endpoints() {
        let mgr = ConfigManager::new();
        assert!(mgr
            .transcription_endpoint()
            .ends_with("/audio/transcriptions"));
        assert!(mgr.chat_endpoint().ends_with("/chat/completions"));
    }
}
