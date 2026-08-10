use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResomerError {
    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Diarization error: {0}")]
    Diarization(String),

    #[error("Transcription error: {0}")]
    Transcription(String),

    #[error("Summarization error: {0}")]
    Summarization(String),

    #[error("Chat error: {0}")]
    Chat(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Rerank error: {0}")]
    Rerank(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Recording error: {0}")]
    RecordingError(String),
}

impl serde::Serialize for ResomerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
