use crate::domain::meeting::{Meeting, MeetingState};
use crate::ResomerError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self, ResomerError> {
        let conn = Connection::open(db_path)
            .map_err(|e| ResomerError::Storage(format!("Failed to open DB: {}", e)))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), ResomerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ResomerError::Storage(format!("Lock failed: {}", e)))?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS meetings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                audio_path TEXT,
                state TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS segments (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                start REAL NOT NULL,
                end REAL NOT NULL,
                speaker TEXT NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE TABLE IF NOT EXISTS transcripts (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL UNIQUE,
                text TEXT NOT NULL,
                language TEXT NOT NULL DEFAULT 'es',
                created_at TEXT NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL UNIQUE,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE INDEX IF NOT EXISTS idx_segments_meeting ON segments(meeting_id);
            CREATE INDEX IF NOT EXISTS idx_meetings_created ON meetings(created_at);
            ",
        )
        .map_err(|e| ResomerError::Storage(format!("Migration failed: {}", e)))?;

        Ok(())
    }

    pub fn save_segments(&self, meeting_id: &str, segments: &[crate::domain::Segment]) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        for segment in segments {
            conn.execute(
                "INSERT INTO segments (meeting_id, start, end, speaker) VALUES (?1, ?2, ?3, ?4)",
                params![meeting_id, segment.start, segment.end, &segment.speaker],
            )
            .map_err(|e| ResomerError::Storage(format!("Insert segment failed: {}", e)))?;
        }

        Ok(())
    }

    pub fn save_transcript(&self, meeting_id: &str, text: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR REPLACE INTO transcripts (meeting_id, text, language, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                meeting_id,
                text,
                "es",
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert transcript failed: {}", e)))?;

        Ok(())
    }

    pub fn save_summary(&self, meeting_id: &str, summary: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR REPLACE INTO summaries (meeting_id, summary, created_at)
             VALUES (?1, ?2, ?3)",
            params![
                meeting_id,
                summary,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert summary failed: {}", e)))?;

        Ok(())
    }

    pub fn get_segments(&self, meeting_id: &str) -> Result<Vec<crate::domain::Segment>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT start, end, speaker FROM segments WHERE meeting_id = ?1 ORDER BY start")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let segments = stmt
            .query_map([meeting_id], |row| {
                Ok(crate::domain::Segment {
                    start: row.get(0)?,
                    end: row.get(1)?,
                    speaker: row.get(2)?,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(segments)
    }

    pub fn get_transcript(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT text FROM transcripts WHERE meeting_id = ?1")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let text = stmt
            .query_row([meeting_id], |row| row.get(0))
            .optional()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(text)
    }

    pub fn get_summary(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT summary FROM summaries WHERE meeting_id = ?1")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let summary = stmt
            .query_row([meeting_id], |row| row.get(0))
            .optional()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(summary)
    }

    pub fn get_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ResomerError> {
        self.conn
            .lock()
            .map_err(|e| ResomerError::Storage(format!("Lock failed: {}", e)))
    }
}

pub struct MeetingRepositoryImpl {
    db: Database,
}

impl MeetingRepositoryImpl {
    pub fn new(db_path: PathBuf) -> Result<Self, ResomerError> {
        Ok(Self {
            db: Database::new(db_path)?,
        })
    }

    pub fn save_segments(&self, meeting_id: &str, segments: &[crate::domain::Segment]) -> Result<(), ResomerError> {
        self.db.save_segments(meeting_id, segments)
    }

    pub fn save_transcript(&self, meeting_id: &str, text: &str) -> Result<(), ResomerError> {
        self.db.save_transcript(meeting_id, text)
    }

    pub fn save_summary(&self, meeting_id: &str, summary: &str) -> Result<(), ResomerError> {
        self.db.save_summary(meeting_id, summary)
    }

    pub fn get_segments(&self, meeting_id: &str) -> Result<Vec<crate::domain::Segment>, ResomerError> {
        self.db.get_segments(meeting_id)
    }

    pub fn get_transcript(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        self.db.get_transcript(meeting_id)
    }

    pub fn get_summary(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        self.db.get_summary(meeting_id)
    }
}

#[async_trait::async_trait]
impl crate::domain::MeetingRepository for MeetingRepositoryImpl {
    async fn create(&self, meeting: Meeting) -> Result<Meeting, ResomerError> {
        let state_str = match &meeting.state {
            MeetingState::Recording => "recording",
            MeetingState::Processing => "processing",
            MeetingState::Completed => "completed",
            MeetingState::Error(_) => "error",
        };

        let conn = self.db.get_connection()?;
        conn.execute(
            "INSERT INTO meetings (id, title, created_at, audio_path, state, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &meeting.id,
                &meeting.title,
                &meeting.created_at,
                &meeting.audio_path,
                state_str,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert failed: {}", e)))?;

        Ok(meeting)
    }

    async fn get(&self, id: &str) -> Result<Option<Meeting>, ResomerError> {
        let conn = self.db.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, audio_path, state FROM meetings WHERE id = ?1")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let meeting = stmt
            .query_row([id], |row| {
                let state_str: String = row.get(4)?;
                let state = match state_str.as_str() {
                    "recording" => MeetingState::Recording,
                    "processing" => MeetingState::Processing,
                    "completed" => MeetingState::Completed,
                    _ => MeetingState::Error("Unknown state".to_string()),
                };

                Ok(Meeting {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    audio_path: row.get(3)?,
                    state,
                })
            })
            .optional()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(meeting)
    }

    async fn list(&self) -> Result<Vec<Meeting>, ResomerError> {
        let conn = self.db.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, audio_path, state FROM meetings ORDER BY created_at DESC")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let meetings = stmt
            .query_map([], |row| {
                let state_str: String = row.get(4)?;
                let state = match state_str.as_str() {
                    "recording" => MeetingState::Recording,
                    "processing" => MeetingState::Processing,
                    "completed" => MeetingState::Completed,
                    _ => MeetingState::Error("Unknown state".to_string()),
                };

                Ok(Meeting {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    audio_path: row.get(3)?,
                    state,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(meetings)
    }

    async fn update(&self, meeting: Meeting) -> Result<Meeting, ResomerError> {
        let state_str = match &meeting.state {
            MeetingState::Recording => "recording",
            MeetingState::Processing => "processing",
            MeetingState::Completed => "completed",
            MeetingState::Error(_) => "error",
        };

        let conn = self.db.get_connection()?;
        conn.execute(
            "UPDATE meetings SET title = ?1, audio_path = ?2, state = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                &meeting.title,
                &meeting.audio_path,
                state_str,
                chrono::Utc::now().to_rfc3339(),
                &meeting.id
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Update failed: {}", e)))?;

        Ok(meeting)
    }

    async fn delete(&self, id: &str) -> Result<(), ResomerError> {
        let conn = self.db.get_connection()?;
        conn.execute("DELETE FROM meetings WHERE id = ?1", [id])
            .map_err(|e| ResomerError::Storage(format!("Delete failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MeetingRepository;
    use std::fs;

    #[tokio::test]
    async fn test_database_crud() {
        let temp_dir = std::env::temp_dir().join("resomer_test_db.sqlite");
        if temp_dir.exists() {
            let _ = fs::remove_file(&temp_dir);
        }

        let repo = MeetingRepositoryImpl::new(temp_dir.clone()).expect("Failed to create repo");

        let meeting = Meeting::new("Test Meeting".to_string());
        let created = repo.create(meeting.clone()).await.expect("Create failed");

        assert_eq!(created.id, meeting.id);
        assert_eq!(created.title, "Test Meeting");

        let retrieved = repo
            .get(&meeting.id)
            .await
            .expect("Get failed")
            .expect("Meeting not found");

        assert_eq!(retrieved.id, meeting.id);

        let _ = fs::remove_file(&temp_dir);
    }
}
