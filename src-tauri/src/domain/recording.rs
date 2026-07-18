use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingSource {
    Microphone,
    SystemAudio,
    Both,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub meeting_id: String,
    pub state: RecordingState,
    pub source: RecordingSource,
    pub file_path: Option<String>,
    pub duration_ms: u64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
}

impl Recording {
    pub fn new(meeting_id: String, source: RecordingSource) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            meeting_id,
            state: RecordingState::Idle,
            source,
            file_path: None,
            duration_ms: 0,
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
        }
    }
}
