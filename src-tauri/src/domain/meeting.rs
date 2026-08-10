use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meeting {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub audio_path: Option<String>,
    pub state: MeetingState,
    /// Participant count the user declared when creating the meeting, pinned
    /// as `num_clusters` during diarization. `None` means "unknown" → the
    /// engine auto-detects. Persisted so a retry keeps the declared count.
    pub expected_speakers: Option<i32>,
    /// Etiqueta libre asignada por el usuario para seccionar la biblioteca de
    /// reuniones (p. ej. "Clientes", "Interno"). `None` = sin categoría.
    pub category: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MeetingState {
    #[serde(rename = "recording")]
    Recording,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "error")]
    Error(String),
}

impl Meeting {
    pub fn new(title: String, expected_speakers: Option<i32>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            created_at: chrono::Utc::now().to_rfc3339(),
            audio_path: None,
            state: MeetingState::Recording,
            expected_speakers,
            category: None,
        }
    }
}
