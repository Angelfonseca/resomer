use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meeting {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub audio_path: Option<String>,
    pub state: MeetingState,
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
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            created_at: chrono::Utc::now().to_rfc3339(),
            audio_path: None,
            state: MeetingState::Recording,
        }
    }
}
