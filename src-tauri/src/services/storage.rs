use crate::domain::meeting::{Meeting, MeetingState};
use crate::services::chat::{ChatMessage, Conversation};
use crate::ResomerError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

/// Un fragmento indexado de la transcripción de una reunión, con su
/// embedding ya calculado — la unidad sobre la que corre la búsqueda
/// semántica cross-meeting.
pub struct ChunkRow {
    pub meeting_id: String,
    pub text: String,
    pub embedding: Vec<f32>,
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
                updated_at TEXT NOT NULL,
                expected_speakers INTEGER
            );

            CREATE TABLE IF NOT EXISTS segments (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                start REAL NOT NULL,
                end REAL NOT NULL,
                speaker TEXT NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE TABLE IF NOT EXISTS utterances (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                start REAL NOT NULL,
                end REAL NOT NULL,
                speaker TEXT NOT NULL,
                text TEXT NOT NULL,
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

            CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE TABLE IF NOT EXISTS global_chat_messages (
                id INTEGER PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS global_conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id)
            );

            CREATE INDEX IF NOT EXISTS idx_segments_meeting ON segments(meeting_id);
            CREATE INDEX IF NOT EXISTS idx_utterances_meeting ON utterances(meeting_id);
            CREATE INDEX IF NOT EXISTS idx_meetings_created ON meetings(created_at);
            CREATE INDEX IF NOT EXISTS idx_chat_meeting ON chat_messages(meeting_id);
            CREATE INDEX IF NOT EXISTS idx_chunks_meeting ON chunks(meeting_id);
            ",
        )
        .map_err(|e| ResomerError::Storage(format!("Migration failed: {}", e)))?;

        // Additive migration for databases created before `expected_speakers`
        // existed. SQLite has no "ADD COLUMN IF NOT EXISTS", so we run it
        // unconditionally and swallow the "duplicate column name" error that
        // fires once the column is already present.
        if let Err(e) = conn.execute(
            "ALTER TABLE meetings ADD COLUMN expected_speakers INTEGER",
            [],
        ) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(ResomerError::Storage(format!("Migration failed: {}", msg)));
            }
        }

        // Mismo patrón: categoría asignable por el usuario para seccionar la
        // biblioteca de reuniones (Clientes, Interno, Personal, ...).
        if let Err(e) = conn.execute("ALTER TABLE meetings ADD COLUMN category TEXT", []) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(ResomerError::Storage(format!("Migration failed: {}", msg)));
            }
        } else {
            // Si la columna se acaba de añadir exitosamente a una base de datos existente,
            // llenamos las categorías antiguas con "Sin categoría" (o dejamos que el frontend lo maneje)
            // En este caso, lo ideal es dejarlas como NULL y que el frontend lo trate como "Sin categoría"
        }

        // Mismo patrón: `global_chat_messages` existía sin `conversation_id`
        // cuando el asistente global tenía una sola conversación eterna.
        if let Err(e) = conn.execute(
            "ALTER TABLE global_chat_messages ADD COLUMN conversation_id TEXT",
            [],
        ) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(ResomerError::Storage(format!("Migration failed: {}", msg)));
            }
        }

        // Backfill: los mensajes anteriores a las conversaciones múltiples no
        // tienen `conversation_id`. Si los hay, se agrupan en una conversación
        // "General" para que no se pierdan al pasar al modelo multi-chat.
        let orphans: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM global_chat_messages WHERE conversation_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if orphans > 0 {
            let conv_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO global_conversations (id, title, created_at) VALUES (?1, ?2, ?3)",
                params![conv_id, "General", chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| ResomerError::Storage(format!("Backfill conversation failed: {}", e)))?;
            conn.execute(
                "UPDATE global_chat_messages SET conversation_id = ?1 WHERE conversation_id IS NULL",
                params![conv_id],
            )
            .map_err(|e| ResomerError::Storage(format!("Backfill messages failed: {}", e)))?;
        }

        Ok(())
    }

    /// Reemplaza los segmentos de diarización de una reunión (borra los
    /// previos primero, igual que `save_utterances`) para que reintentar un
    /// guardado fallido, o reprocesar, no duplique filas.
    pub fn save_segments(
        &self,
        meeting_id: &str,
        segments: &[crate::domain::Segment],
    ) -> Result<(), ResomerError> {
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| ResomerError::Storage(format!("Transaction failed: {}", e)))?;

        tx.execute("DELETE FROM segments WHERE meeting_id = ?1", [meeting_id])
            .map_err(|e| ResomerError::Storage(format!("Delete segments failed: {}", e)))?;

        for segment in segments {
            tx.execute(
                "INSERT INTO segments (meeting_id, start, end, speaker) VALUES (?1, ?2, ?3, ?4)",
                params![meeting_id, segment.start, segment.end, &segment.speaker],
            )
            .map_err(|e| ResomerError::Storage(format!("Insert segment failed: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| ResomerError::Storage(format!("Commit failed: {}", e)))?;

        Ok(())
    }

    pub fn save_transcript(&self, meeting_id: &str, text: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR REPLACE INTO transcripts (meeting_id, text, language, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![meeting_id, text, "es", chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert transcript failed: {}", e)))?;

        Ok(())
    }

    pub fn save_summary(&self, meeting_id: &str, summary: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR REPLACE INTO summaries (meeting_id, summary, created_at)
             VALUES (?1, ?2, ?3)",
            params![meeting_id, summary, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert summary failed: {}", e)))?;

        Ok(())
    }

    pub fn get_segments(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<crate::domain::Segment>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT start, end, speaker FROM segments WHERE meeting_id = ?1 ORDER BY start",
            )
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

    /// Reemplaza las intervenciones atribuidas de una reunión (borra las
    /// previas para no mezclar resultados de un reproceso con los anteriores).
    pub fn save_utterances(
        &self,
        meeting_id: &str,
        utterances: &[crate::domain::SpeakerUtterance],
    ) -> Result<(), ResomerError> {
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| ResomerError::Storage(format!("Transaction failed: {}", e)))?;

        tx.execute("DELETE FROM utterances WHERE meeting_id = ?1", [meeting_id])
            .map_err(|e| ResomerError::Storage(format!("Delete utterances failed: {}", e)))?;

        for u in utterances {
            tx.execute(
                "INSERT INTO utterances (meeting_id, start, end, speaker, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![meeting_id, u.start, u.end, &u.speaker, &u.text],
            )
            .map_err(|e| ResomerError::Storage(format!("Insert utterance failed: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| ResomerError::Storage(format!("Commit failed: {}", e)))?;

        Ok(())
    }

    pub fn get_utterances(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<crate::domain::SpeakerUtterance>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT speaker, start, end, text FROM utterances WHERE meeting_id = ?1 ORDER BY start",
            )
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let utterances = stmt
            .query_map([meeting_id], |row| {
                Ok(crate::domain::SpeakerUtterance {
                    speaker: row.get(0)?,
                    start: row.get(1)?,
                    end: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(utterances)
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

    pub fn save_chat_message(
        &self,
        meeting_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT INTO chat_messages (meeting_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![meeting_id, role, content, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert chat message failed: {}", e)))?;

        Ok(())
    }

    /// Se llama desde la pantalla de grabación una vez el usuario elige (o
    /// declina elegir) el número de participantes — momento posterior a la
    /// creación de la reunión, así que es una actualización separada en vez
    /// de ir en el INSERT inicial.
    pub fn set_expected_speakers(
        &self,
        meeting_id: &str,
        expected_speakers: Option<i32>,
    ) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE meetings SET expected_speakers = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                expected_speakers,
                chrono::Utc::now().to_rfc3339(),
                meeting_id
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Update expected_speakers failed: {}", e)))?;

        Ok(())
    }

    pub fn get_chat_history(&self, meeting_id: &str) -> Result<Vec<ChatMessage>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT role, content, created_at FROM chat_messages
                 WHERE meeting_id = ?1 ORDER BY id ASC",
            )
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let messages = stmt
            .query_map([meeting_id], |row| {
                Ok(ChatMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(messages)
    }

    /// El historial del asistente global vive en su propia tabla, sin
    /// `meeting_id` ni FOREIGN KEY: no está atado a ninguna reunión
    /// concreta, y SQLite (con el build "bundled" de rusqlite) sí hace
    /// cumplir las claves foráneas, así que no puede reusar `chat_messages`
    /// con un id inventado.
    pub fn save_global_chat_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT INTO global_chat_messages (conversation_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![conversation_id, role, content, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert global chat message failed: {}", e)))?;

        Ok(())
    }

    pub fn get_global_chat_history(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT role, content, created_at FROM global_chat_messages
                 WHERE conversation_id = ?1 ORDER BY id ASC",
            )
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let messages = stmt
            .query_map([conversation_id], |row| {
                Ok(ChatMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(messages)
    }

    /// Conversaciones del asistente global, más recientes primero.
    pub fn list_conversations(&self) -> Result<Vec<Conversation>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT id, title, created_at FROM global_conversations ORDER BY created_at DESC")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let convs = stmt
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(convs)
    }

    pub fn create_conversation(&self, conv: &Conversation) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO global_conversations (id, title, created_at) VALUES (?1, ?2, ?3)",
            params![conv.id, conv.title, conv.created_at],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert conversation failed: {}", e)))?;
        Ok(())
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE global_conversations SET title = ?1 WHERE id = ?2",
            params![title, id],
        )
        .map_err(|e| ResomerError::Storage(format!("Rename conversation failed: {}", e)))?;
        Ok(())
    }

    /// Título actual de una conversación (para saber si sigue siendo el
    /// placeholder y auto-nombrarla con la primera pregunta).
    pub fn get_conversation_title(&self, id: &str) -> Result<Option<String>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT title FROM global_conversations WHERE id = ?1")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;
        stmt.query_row([id], |row| row.get(0))
            .optional()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), ResomerError> {
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| ResomerError::Storage(format!("Transaction failed: {}", e)))?;
        tx.execute(
            "DELETE FROM global_chat_messages WHERE conversation_id = ?1",
            [id],
        )
        .map_err(|e| ResomerError::Storage(format!("Delete messages failed: {}", e)))?;
        tx.execute("DELETE FROM global_conversations WHERE id = ?1", [id])
            .map_err(|e| ResomerError::Storage(format!("Delete conversation failed: {}", e)))?;
        tx.commit()
            .map_err(|e| ResomerError::Storage(format!("Commit failed: {}", e)))?;
        Ok(())
    }

    /// Renombra una reunión (título editable / generado por IA). Como
    /// `set_expected_speakers`, es una actualización puntual de una columna en
    /// vez de reescribir toda la fila con `update`.
    pub fn set_meeting_title(&self, meeting_id: &str, title: &str) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE meetings SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, chrono::Utc::now().to_rfc3339(), meeting_id],
        )
        .map_err(|e| ResomerError::Storage(format!("Update meeting title failed: {}", e)))?;
        Ok(())
    }

    pub fn set_meeting_category(
        &self,
        meeting_id: &str,
        category: Option<&str>,
    ) -> Result<(), ResomerError> {
        let conn = self.get_connection()?;
        let affected = conn.execute(
            "UPDATE meetings SET category = ?1, updated_at = ?2 WHERE id = ?3",
            params![category, chrono::Utc::now().to_rfc3339(), meeting_id],
        )
        .map_err(|e| ResomerError::Storage(format!("Update meeting category failed: {}", e)))?;
        
        if affected == 0 {
            // El ID probablemente no existe en BD todavía (ej. "local-12345")
            // No lo consideramos un error fatal, el frontend usa actualización optimista
            // y cuando se cree realmente la reunión se guardará con su categoría correcta
        }
        Ok(())
    }

    /// Serializa un vector de embedding a bytes little-endian para guardarlo
    /// como BLOB — evita añadir una dependencia (bytemuck) solo para esto.
    fn encode_embedding(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn decode_embedding(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// Reemplaza los chunks indexados de una reunión (usado tras generar una
    /// nueva transcripción, incluyendo reprocesos: se borran los anteriores
    /// para no dejar embeddings obsoletos mezclados con los nuevos).
    pub fn save_chunks(
        &self,
        meeting_id: &str,
        chunks: &[(String, Vec<f32>)],
    ) -> Result<(), ResomerError> {
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| ResomerError::Storage(format!("Transaction failed: {}", e)))?;

        tx.execute("DELETE FROM chunks WHERE meeting_id = ?1", [meeting_id])
            .map_err(|e| ResomerError::Storage(format!("Delete chunks failed: {}", e)))?;

        for (i, (text, embedding)) in chunks.iter().enumerate() {
            tx.execute(
                "INSERT INTO chunks (meeting_id, chunk_index, text, embedding) VALUES (?1, ?2, ?3, ?4)",
                params![meeting_id, i as i64, text, Self::encode_embedding(embedding)],
            )
            .map_err(|e| ResomerError::Storage(format!("Insert chunk failed: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| ResomerError::Storage(format!("Commit failed: {}", e)))?;

        Ok(())
    }

    /// Trae todos los chunks indexados de todas las reuniones, para la
    /// búsqueda semántica cross-meeting (similitud coseno en memoria — a la
    /// escala de una app de escritorio local, esto es más simple y suficiente
    /// que una extensión de vectores en SQLite).
    pub fn get_all_chunks(&self) -> Result<Vec<ChunkRow>, ResomerError> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT meeting_id, text, embedding FROM chunks")
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?;

        let chunks = stmt
            .query_map([], |row| {
                let embedding_bytes: Vec<u8> = row.get(2)?;
                Ok(ChunkRow {
                    meeting_id: row.get(0)?,
                    text: row.get(1)?,
                    embedding: Self::decode_embedding(&embedding_bytes),
                })
            })
            .map_err(|e| ResomerError::Storage(format!("Query failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(chunks)
    }

    /// Borra por completo todo lo asociado a una reunión (segmentos,
    /// transcripción, resumen, historial de chat) y finalmente la propia
    /// reunión, en una sola transacción. No borra el archivo de audio en
    /// disco — eso lo maneja la capa de comandos (lib.rs), que sí conoce la
    /// ruta y puede decidir qué hacer si el borrado del archivo falla.
    pub fn delete_meeting_cascade(&self, meeting_id: &str) -> Result<(), ResomerError> {
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| ResomerError::Storage(format!("Transaction failed: {}", e)))?;

        for table in [
            "segments",
            "utterances",
            "transcripts",
            "summaries",
            "chat_messages",
            "chunks",
        ] {
            tx.execute(
                &format!("DELETE FROM {} WHERE meeting_id = ?1", table),
                [meeting_id],
            )
            .map_err(|e| ResomerError::Storage(format!("Delete from {} failed: {}", table, e)))?;
        }

        tx.execute("DELETE FROM meetings WHERE id = ?1", [meeting_id])
            .map_err(|e| ResomerError::Storage(format!("Delete meeting failed: {}", e)))?;

        tx.commit()
            .map_err(|e| ResomerError::Storage(format!("Commit failed: {}", e)))?;

        Ok(())
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

    pub fn save_segments(
        &self,
        meeting_id: &str,
        segments: &[crate::domain::Segment],
    ) -> Result<(), ResomerError> {
        self.db.save_segments(meeting_id, segments)
    }

    pub fn save_transcript(&self, meeting_id: &str, text: &str) -> Result<(), ResomerError> {
        self.db.save_transcript(meeting_id, text)
    }

    pub fn save_summary(&self, meeting_id: &str, summary: &str) -> Result<(), ResomerError> {
        self.db.save_summary(meeting_id, summary)
    }

    pub fn get_segments(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<crate::domain::Segment>, ResomerError> {
        self.db.get_segments(meeting_id)
    }

    pub fn save_utterances(
        &self,
        meeting_id: &str,
        utterances: &[crate::domain::SpeakerUtterance],
    ) -> Result<(), ResomerError> {
        self.db.save_utterances(meeting_id, utterances)
    }

    pub fn get_utterances(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<crate::domain::SpeakerUtterance>, ResomerError> {
        self.db.get_utterances(meeting_id)
    }

    pub fn get_transcript(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        self.db.get_transcript(meeting_id)
    }

    pub fn get_summary(&self, meeting_id: &str) -> Result<Option<String>, ResomerError> {
        self.db.get_summary(meeting_id)
    }

    pub fn set_expected_speakers(
        &self,
        meeting_id: &str,
        expected_speakers: Option<i32>,
    ) -> Result<(), ResomerError> {
        self.db.set_expected_speakers(meeting_id, expected_speakers)
    }

    pub fn save_chunks(
        &self,
        meeting_id: &str,
        chunks: &[(String, Vec<f32>)],
    ) -> Result<(), ResomerError> {
        self.db.save_chunks(meeting_id, chunks)
    }

    pub fn get_all_chunks(&self) -> Result<Vec<ChunkRow>, ResomerError> {
        self.db.get_all_chunks()
    }

    pub fn save_chat_message(
        &self,
        meeting_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), ResomerError> {
        self.db.save_chat_message(meeting_id, role, content)
    }

    pub fn save_global_chat_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), ResomerError> {
        self.db
            .save_global_chat_message(conversation_id, role, content)
    }

    pub fn get_global_chat_history(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, ResomerError> {
        self.db.get_global_chat_history(conversation_id)
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, ResomerError> {
        self.db.list_conversations()
    }

    pub fn create_conversation(&self, conv: &Conversation) -> Result<(), ResomerError> {
        self.db.create_conversation(conv)
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> Result<(), ResomerError> {
        self.db.rename_conversation(id, title)
    }

    pub fn get_conversation_title(&self, id: &str) -> Result<Option<String>, ResomerError> {
        self.db.get_conversation_title(id)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), ResomerError> {
        self.db.delete_conversation(id)
    }

    pub fn set_meeting_title(&self, meeting_id: &str, title: &str) -> Result<(), ResomerError> {
        self.db.set_meeting_title(meeting_id, title)
    }

    pub fn set_meeting_category(
        &self,
        meeting_id: &str,
        category: Option<&str>,
    ) -> Result<(), ResomerError> {
        self.db.set_meeting_category(meeting_id, category)
    }

    pub fn get_chat_history(&self, meeting_id: &str) -> Result<Vec<ChatMessage>, ResomerError> {
        self.db.get_chat_history(meeting_id)
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
            "INSERT INTO meetings (id, title, created_at, audio_path, state, updated_at, expected_speakers, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &meeting.id,
                &meeting.title,
                &meeting.created_at,
                &meeting.audio_path,
                state_str,
                chrono::Utc::now().to_rfc3339(),
                &meeting.expected_speakers,
                &meeting.category
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Insert failed: {}", e)))?;

        Ok(meeting)
    }

    async fn get(&self, id: &str) -> Result<Option<Meeting>, ResomerError> {
        let conn = self.db.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, audio_path, state, expected_speakers, category FROM meetings WHERE id = ?1")
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
                    expected_speakers: row.get(5)?,
                    category: row.get(6)?,
                })
            })
            .optional()
            .map_err(|e| ResomerError::Storage(format!("Row conversion failed: {}", e)))?;

        Ok(meeting)
    }

    async fn list(&self) -> Result<Vec<Meeting>, ResomerError> {
        let conn = self.db.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, audio_path, state, expected_speakers, category FROM meetings ORDER BY created_at DESC")
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
                    expected_speakers: row.get(5)?,
                    category: row.get(6)?,
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
            "UPDATE meetings SET title = ?1, audio_path = ?2, state = ?3, updated_at = ?4, expected_speakers = ?5, category = ?6
             WHERE id = ?7",
            params![
                &meeting.title,
                &meeting.audio_path,
                state_str,
                chrono::Utc::now().to_rfc3339(),
                &meeting.expected_speakers,
                &meeting.category,
                &meeting.id
            ],
        )
        .map_err(|e| ResomerError::Storage(format!("Update failed: {}", e)))?;

        Ok(meeting)
    }

    async fn delete(&self, id: &str) -> Result<(), ResomerError> {
        self.db.delete_meeting_cascade(id)
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

        let meeting = Meeting::new("Test Meeting".to_string(), None);
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

    /// Garantiza que abrir una BD con el esquema VIEJO (global_chat_messages
    /// sin `conversation_id`, como la de un usuario ya existente) no rompe nada:
    /// la migración añade la columna y conserva los mensajes previos bajo una
    /// conversación "General".
    #[test]
    fn migration_preserves_old_global_chat() {
        let path = std::env::temp_dir().join("resomer_migration_test.sqlite");
        let _ = fs::remove_file(&path);

        // Simula el esquema anterior a las conversaciones múltiples.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE global_chat_messages (
                    id INTEGER PRIMARY KEY,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                 );
                 INSERT INTO global_chat_messages (role, content, created_at)
                    VALUES ('user', 'pregunta vieja', '2024-01-01T00:00:00Z');
                 INSERT INTO global_chat_messages (role, content, created_at)
                    VALUES ('assistant', 'respuesta vieja', '2024-01-01T00:00:01Z');",
            )
            .unwrap();
        }

        // Al abrir con el código nuevo, migrate() corre el backfill.
        let db = Database::new(path.clone()).expect("migration failed");

        let convs = db.list_conversations().expect("list failed");
        assert_eq!(convs.len(), 1, "debería crearse una conversación 'General'");
        assert_eq!(convs[0].title, "General");

        let history = db
            .get_global_chat_history(&convs[0].id)
            .expect("history failed");
        assert_eq!(history.len(), 2, "los mensajes previos deben conservarse");
        assert_eq!(history[0].content, "pregunta vieja");

        // Idempotente: reabrir no vuelve a crear otra "General".
        drop(db);
        let db2 = Database::new(path.clone()).expect("reopen failed");
        assert_eq!(db2.list_conversations().unwrap().len(), 1);

        let _ = fs::remove_file(&path);
    }
}
