use crate::domain::{Meeting, Segment};
use crate::ResomerError;

// Trait definitions (ports) — implementadas por services/infra
// Una responsabilidad cada uno, injectadas en comandos

#[async_trait::async_trait]
pub trait AudioRecorder: Send + Sync {
    async fn start_recording(&self, output_path: &str) -> Result<(), ResomerError>;
    async fn stop_recording(&self) -> Result<(), ResomerError>;
    async fn pause_recording(&self) -> Result<(), ResomerError>;
}

#[async_trait::async_trait]
pub trait DiarizationEngine: Send + Sync {
    async fn diarize(
        &self,
        audio_path: &str,
        num_speakers: Option<i32>,
    ) -> Result<Vec<Segment>, ResomerError>;
}

#[async_trait::async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(&self, audio_path: &str) -> Result<String, ResomerError>;
}

#[async_trait::async_trait]
pub trait Summarizer: Send + Sync {
    async fn summarize(&self, text: &str) -> Result<String, ResomerError>;
}

#[async_trait::async_trait]
pub trait MeetingRepository: Send + Sync {
    async fn create(&self, meeting: Meeting) -> Result<Meeting, ResomerError>;
    async fn get(&self, id: &str) -> Result<Option<Meeting>, ResomerError>;
    async fn list(&self) -> Result<Vec<Meeting>, ResomerError>;
    async fn update(&self, meeting: Meeting) -> Result<Meeting, ResomerError>;
    async fn delete(&self, id: &str) -> Result<(), ResomerError>;
}
