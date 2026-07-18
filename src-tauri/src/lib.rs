// pub mod commands; // Commands are defined directly in lib.rs for Tauri macro
pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

pub use error::ResomerError;

// Command implementations (without Tauri annotations - those go in main.rs only)
use domain::meeting::{Meeting, MeetingState};
use domain::{AudioRecorder, DiarizationEngine, RecordingSource, Segment, Summarizer, Transcriber, MeetingRepository};
use infra::config::ConfigManager;
use infra::keychain::KeychainManager;
use serde::{Deserialize, Serialize};
use services::{
    list_input_devices, CloudTranscriber, CpalAudioRecorder, LlmSummarizer, SherpaDiarizationEngine,
    MeetingRepositoryImpl, SystemAudioRecorder,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

pub fn save_api_key(api_key: String) -> Result<(), String> {
    KeychainManager::save_api_key(&api_key).map_err(|e| e.to_string())
}

pub fn get_api_key() -> Result<Option<String>, String> {
    KeychainManager::get_api_key().map_err(|e| e.to_string())
}

pub fn delete_api_key() -> Result<(), String> {
    KeychainManager::delete_api_key().map_err(|e| e.to_string())
}

pub async fn test_connection(api_key: String) -> Result<TestConnectionResponse, String> {
    let config = ConfigManager::new();
    let endpoint = config.chat_endpoint();

    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "mimo-v2.5",
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 10
        }))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() {
        Ok(TestConnectionResponse {
            success: true,
            message: "Connection OK".to_string(),
        })
    } else {
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Server returned: {}", response.status()),
        })
    }
}

pub async fn create_meeting(title: String) -> Result<Meeting, String> {
    let meeting = Meeting::new(title);
    let repo = get_database()?;
    repo.create(meeting).await.map_err(|e| e.to_string())
}

// Global recorder instance - initialized once per app
type RecorderState = Arc<Mutex<Option<Arc<dyn AudioRecorder>>>>;
pub static RECORDER: std::sync::OnceLock<RecorderState> = std::sync::OnceLock::new();

// Global database instance - initialized once per app
pub static DATABASE: std::sync::OnceLock<Arc<MeetingRepositoryImpl>> = std::sync::OnceLock::new();

pub fn get_database() -> Result<Arc<MeetingRepositoryImpl>, String> {
    if let Some(db) = DATABASE.get() {
        return Ok(db.clone());
    }

    // Try to use home directory, fallback to current directory
    let db_dir = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".resomer").join("data")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile).join(".resomer").join("data")
    } else {
        PathBuf::from(".").join(".resomer").join("data")
    };

    // Create parent directories if they don't exist
    let _ = std::fs::create_dir_all(&db_dir);

    let db_path = db_dir.join("resomer.db");

    let repo = Arc::new(
        MeetingRepositoryImpl::new(db_path)
            .map_err(|e| format!("Failed to initialize database: {}", e))?
    );

    DATABASE
        .set(repo.clone())
        .map_err(|_| "Failed to set database".to_string())?;

    Ok(repo)
}

/// Carpeta de grabaciones escribible (`~/.resomer/recordings`). Se crea si no
/// existe. Nunca depende del directorio de trabajo, que en la app empaquetada
/// es de solo lectura.
fn recordings_dir() -> Result<PathBuf, String> {
    let base = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".resomer").join("recordings")
    } else if let Ok(up) = std::env::var("USERPROFILE") {
        PathBuf::from(up).join(".resomer").join("recordings")
    } else {
        PathBuf::from(".").join(".resomer").join("recordings")
    };
    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    Ok(base)
}

/// Convierte la ruta recibida del frontend en una ruta absoluta escribible.
/// Si es relativa, usa solo el nombre de archivo bajo `recordings_dir()`.
fn resolve_recording_path(output_path: &str) -> Result<String, String> {
    let p = PathBuf::from(output_path);
    let abs = if p.is_absolute() {
        p
    } else {
        let name = p.file_name().map(PathBuf::from).unwrap_or(p);
        recordings_dir()?.join(name)
    };
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    }
    abs.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Ruta de grabación inválida".to_string())
}

pub async fn start_recording(
    app: tauri::AppHandle,
    meeting_id: String,
    output_path: String,
    source: String,
) -> Result<String, String> {
    let recording_source = match source.as_str() {
        "microphone" => RecordingSource::Microphone,
        "system_audio" => RecordingSource::SystemAudio,
        "both" => RecordingSource::Both,
        _ => return Err("Invalid recording source".to_string()),
    };

    // Resolver a una ruta absoluta escribible (crea la carpeta si hace falta).
    let resolved_path = resolve_recording_path(&output_path)?;

    // Update meeting in BD with audio path (or create if doesn't exist)
    if let Ok(repo) = get_database() {
        match repo.get(&meeting_id).await {
            Ok(Some(mut meeting)) => {
                meeting.audio_path = Some(resolved_path.clone());
                meeting.state = MeetingState::Recording;
                let _ = repo.update(meeting).await;
            }
            _ => {
                // Create meeting if it doesn't exist
                let mut meeting = Meeting::new("Reunión".to_string());
                meeting.id = meeting_id.clone();
                meeting.audio_path = Some(resolved_path.clone());
                meeting.state = MeetingState::Recording;
                let _ = repo.create(meeting).await;
            }
        }
    }

    // Select appropriate recorder based on source
    let recorder: Arc<dyn AudioRecorder> = match recording_source {
        RecordingSource::Microphone => {
            Arc::new(CpalAudioRecorder::new().map_err(|e| format!("Recorder init failed: {}", e))?)
        }
        RecordingSource::SystemAudio => {
            // Solo audio del sistema (ScreenCaptureKit).
            Arc::new(
                SystemAudioRecorder::with_options(app.clone(), false)
                    .map_err(|e| format!("System audio recorder init failed: {}", e))?,
            )
        }
        RecordingSource::Both => {
            // Audio del sistema + micrófono mezclados en una sola pista.
            Arc::new(
                SystemAudioRecorder::with_options(app.clone(), true)
                    .map_err(|e| format!("System audio recorder init failed: {}", e))?,
            )
        }
    };

    recorder
        .start_recording(&resolved_path)
        .await
        .map_err(|e| e.to_string())?;

    // Store recorder in global state for later use
    let recorder_state = RECORDER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut state = recorder_state.lock().await;
    *state = Some(recorder);

    Ok(resolved_path)
}

pub async fn stop_recording(meeting_id: String) -> Result<(), String> {
    let recorder_state = RECORDER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut state = recorder_state.lock().await;

    if let Some(recorder) = state.as_ref() {
        recorder.stop_recording().await.map_err(|e| e.to_string())?;
    }

    *state = None;

    // La grabación terminó; el audio está en disco pero el pipeline
    // (diarización/transcripción/resumen) aún no corrió.
    if let Ok(repo) = get_database() {
        if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
            meeting.state = MeetingState::Processing;
            let _ = repo.update(meeting).await;
        }
    }

    Ok(())
}

pub async fn pause_recording() -> Result<(), String> {
    let recorder_state = RECORDER.get_or_init(|| Arc::new(Mutex::new(None)));
    let state = recorder_state.lock().await;

    if let Some(recorder) = state.as_ref() {
        recorder
            .pause_recording()
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn list_audio_devices() -> Result<Vec<services::devices::AudioDevice>, String> {
    list_input_devices().map_err(|e| e.to_string())
}

pub async fn diarize_audio(audio_path: String) -> Result<Vec<Segment>, String> {
    let engine = SherpaDiarizationEngine::new();
    engine.diarize(&audio_path).await.map_err(|e| e.to_string())
}

pub async fn transcribe_audio(
    audio_path: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    // Obtener API key del keychain
    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let transcriber = CloudTranscriber::new(api_endpoint, api_key, model);
    transcriber
        .transcribe(&audio_path)
        .await
        .map_err(|e| e.to_string())
}

pub async fn summarize_text(
    text: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    // Obtener API key del keychain
    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let summarizer = LlmSummarizer::new(api_endpoint, api_key, model);
    summarizer.summarize(&text).await.map_err(|e| e.to_string())
}

pub async fn save_pipeline_results(
    meeting_id: String,
    segments: Vec<Segment>,
    transcript: String,
    summary: String,
) -> Result<(), String> {
    let repo = get_database()?;

    repo.save_segments(&meeting_id, &segments)
        .map_err(|e| e.to_string())?;

    repo.save_transcript(&meeting_id, &transcript)
        .map_err(|e| e.to_string())?;

    repo.save_summary(&meeting_id, &summary)
        .map_err(|e| e.to_string())?;

    // El pipeline completó con éxito: marcar la reunión como completada.
    if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
        meeting.state = MeetingState::Completed;
        let _ = repo.update(meeting).await;
    }

    Ok(())
}

/// Marca una reunión como fallida cuando el pipeline (diarización,
/// transcripción o resumen) aborta con error, para que deje de mostrarse
/// como "grabando"/"procesando" indefinidamente en la interfaz.
pub async fn mark_meeting_error(meeting_id: String, error: String) -> Result<(), String> {
    let repo = get_database()?;
    if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
        meeting.state = MeetingState::Error(error);
        let _ = repo.update(meeting).await;
    }
    Ok(())
}

pub async fn delete_meeting(meeting_id: String) -> Result<(), String> {
    let repo = get_database()?;
    repo.delete(&meeting_id).await.map_err(|e| e.to_string())
}

pub async fn get_meeting_data(
    meeting_id: String,
) -> Result<MeetingData, String> {
    let repo = get_database()?;

    let segments = repo.get_segments(&meeting_id)
        .map_err(|e| e.to_string())?;

    let transcript = repo.get_transcript(&meeting_id)
        .map_err(|e| e.to_string())?;

    let summary = repo.get_summary(&meeting_id)
        .map_err(|e| e.to_string())?;

    Ok(MeetingData {
        segments,
        transcript,
        summary,
    })
}

pub async fn list_meetings() -> Result<Vec<Meeting>, String> {
    let repo = get_database()?;
    repo.list().await.map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct MeetingData {
    pub segments: Vec<Segment>,
    pub transcript: Option<String>,
    pub summary: Option<String>,
}
