// pub mod commands; // Commands are defined directly in lib.rs for Tauri macro
pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

pub use error::ResomerError;

// Command implementations (without Tauri annotations - those go in main.rs only)
use domain::meeting::{Meeting, MeetingState};
use domain::{
    AudioRecorder, DiarizationEngine, MeetingRepository, RecordingSource, Segment, SpeakerUtterance,
};
use infra::config::ConfigManager;
use infra::keychain::KeychainManager;
use serde::{Deserialize, Serialize};
use services::{
    attribute_speakers, list_input_devices, CloudTranscriber, CpalAudioRecorder, LlmSummarizer,
    MeetingRepositoryImpl, SherpaDiarizationEngine, SystemAudioRecorder, TranscriptSegment,
    TranscriptionOutput,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

/// Estado de la API key para la UI: nunca exponemos el secreto al webview, solo
/// si está configurada y una versión enmascarada para mostrarla.
#[derive(Serialize, Deserialize)]
pub struct ApiKeyStatus {
    pub configured: bool,
    pub masked: Option<String>,
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 6 {
        return "•".repeat(key.len());
    }
    let bullets = "•".repeat((key.len() - 6).max(3));
    format!("{}{}{}", &key[..3], bullets, &key[key.len() - 3..])
}

pub fn save_api_key(api_key: String) -> Result<(), String> {
    KeychainManager::save_api_key(&api_key).map_err(|e| e.to_string())
}

/// Devuelve solo el estado (configurada + enmascarada). La clave en claro nunca
/// cruza al frontend, así un script inyectado no puede leerla.
pub fn get_api_key_status() -> Result<ApiKeyStatus, String> {
    let key = KeychainManager::get_api_key().map_err(|e| e.to_string())?;
    Ok(ApiKeyStatus {
        configured: key.is_some(),
        masked: key.as_deref().map(mask_api_key),
    })
}

pub fn delete_api_key() -> Result<(), String> {
    KeychainManager::delete_api_key().map_err(|e| e.to_string())
}

/// Prueba la conexión con la clave guardada en el Keychain y el endpoint
/// configurado en el backend. No acepta una clave ni un endpoint del frontend:
/// así una webview comprometida no puede usar esta llamada para filtrar la
/// clave a un host arbitrario.
pub async fn test_connection() -> Result<TestConnectionResponse, String> {
    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "No hay ninguna clave configurada para probar.".to_string())?;

    let config = ConfigManager::new();
    let endpoint = config.chat_endpoint();

    let client = infra::http_client::build_client();
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

pub async fn create_meeting(
    title: String,
    expected_speakers: Option<i32>,
    category: Option<String>,
) -> Result<Meeting, String> {
    let mut meeting = Meeting::new(title, expected_speakers);
    meeting.category = category;

    let repo = get_database()?;
    repo.create(meeting).await.map_err(|e| e.to_string())
}

/// La grabación en curso, si la hay. Agrupa en un único sitio el grabador y
/// los metadatos de la reunión: antes eran dos `OnceLock`s independientes
/// (`RECORDER` + `CURRENT_RECORDING`) con un check-then-set no atómico, lo que
/// permitía dos grabaciones concurrentes y grabadores huérfanos. Un solo lock
/// hace de fuente de verdad y serializa start/stop/pause.
pub struct RecordingSession {
    pub meeting_id: String,
    pub audio_path: String,
    pub source: String,
    pub recorder: Arc<dyn AudioRecorder>,
}

static SESSION: std::sync::OnceLock<Mutex<Option<RecordingSession>>> = std::sync::OnceLock::new();

fn session_slot() -> &'static Mutex<Option<RecordingSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// Grabación activa (id, ruta, fuente), o `None` si no hay ninguna. Lo usa el
/// frontend al abrirse para reflejar una grabación iniciada desde el tray.
pub async fn get_active_recording() -> Option<ActiveRecording> {
    session_slot()
        .lock()
        .await
        .as_ref()
        .map(|s| ActiveRecording {
            meeting_id: s.meeting_id.clone(),
            audio_path: s.audio_path.clone(),
            source: s.source.clone(),
        })
}

/// Detiene de forma síncrona cualquier grabación activa y suelta el grabador.
/// Se llama en el evento de salida de la app: al caer el `Arc<dyn
/// AudioRecorder>` se dispara el `Drop` del helper de audio del sistema, que
/// mata el proceso hijo. Sin esto, cerrar la app durante una grabación de
/// sistema dejaba el helper capturando audio y el WAV corrupto.
pub fn shutdown_recording() {
    tauri::async_runtime::block_on(async {
        let mut slot = session_slot().lock().await;
        let _ = slot.take();
    });
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRecording {
    pub meeting_id: String,
    pub audio_path: String,
    pub source: String,
}

/// Inicia una grabación directamente desde el ícono de la barra de estado, sin
/// pasar por la UI: crea la reunión, genera la ruta y arranca el grabador.
/// Devuelve (id, ruta absoluta) para que el frontend, si está abierto, refleje
/// la grabación en curso.
pub async fn tray_start_recording(
    app: tauri::AppHandle,
    source: String,
) -> Result<ActiveRecording, String> {
    if session_slot().lock().await.is_some() {
        return Err("Ya hay una grabación en curso".to_string());
    }

    let meeting = Meeting::new("Reunión".to_string(), None);
    let meeting_id = meeting.id.clone();
    if let Ok(repo) = get_database() {
        let _ = repo.create(meeting).await;
    }

    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let output_path = format!("{}-{}.wav", meeting_id, timestamp);

    let audio_path = start_recording(app, meeting_id.clone(), output_path, source.clone()).await?;
    Ok(ActiveRecording {
        meeting_id,
        audio_path,
        source,
    })
}

/// Detiene la grabación activa (si la hay), sea cual sea su origen. La usa el
/// ícono de la barra de estado para parar sin abrir la app. Devuelve la
/// reunión detenida para que el frontend abra su pipeline.
pub async fn tray_stop_recording() -> Result<Option<ActiveRecording>, String> {
    let active = get_active_recording().await;
    if let Some(a) = active {
        stop_recording(a.meeting_id.clone()).await?;
        Ok(Some(a))
    } else {
        Ok(None)
    }
}

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
            .map_err(|e| format!("Failed to initialize database: {}", e))?,
    );

    // Si dos comandos inicializan la BD a la vez, el que pierde la carrera no
    // debe fallar: devuelve la instancia que sí quedó publicada.
    match DATABASE.set(repo.clone()) {
        Ok(()) => Ok(repo),
        Err(_) => DATABASE
            .get()
            .cloned()
            .ok_or_else(|| "Failed to initialize database".to_string()),
    }
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

/// Convierte la ruta recibida del frontend en una ruta absoluta escribible
/// **dentro** de `~/.resomer/recordings`. Se rechaza cualquier ruta que escape
/// de esa carpeta, para que una webview comprometida no pueda hacer que la app
/// escriba un WAV en una ubicación arbitraria.
fn resolve_recording_path(output_path: &str) -> Result<String, String> {
    let base = recordings_dir()?;
    let p = PathBuf::from(output_path);
    let candidate = if p.is_absolute() {
        p
    } else {
        let name = p.file_name().map(PathBuf::from).unwrap_or(p);
        base.join(name)
    };

    let parent = candidate
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.clone());
    std::fs::create_dir_all(&parent)
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;

    // El archivo aún no existe, así que canonicalizamos el directorio padre.
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("Invalid recordings directory: {}", e))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Invalid recording path: {}", e))?;
    if !canonical_parent.starts_with(&canonical_base) {
        return Err("Ruta de grabación fuera de la carpeta permitida".to_string());
    }

    candidate
        .to_str()
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

    // Serializa el arranque: el lock se mantiene durante todo el arranque (que
    // puede tardar en audio de sistema) para que dos peticiones simultáneas no
    // puedan pasar ambas el chequeo `is_some`.
    let mut slot = session_slot().lock().await;
    if slot.is_some() {
        return Err("Ya hay una grabación en curso".to_string());
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

    tracing::info!(meeting_id = %meeting_id, source = %source, "grabación iniciada");

    // Persistir SOLO tras arrancar bien: si el arranque falla, la reunión no
    // queda marcada "grabando" apuntando a un archivo que nunca se creó.
    if let Ok(repo) = get_database() {
        match repo.get(&meeting_id).await {
            Ok(Some(mut meeting)) => {
                meeting.audio_path = Some(resolved_path.clone());
                meeting.state = MeetingState::Recording;
                let _ = repo.update(meeting).await;
            }
            _ => {
                // Create meeting if it doesn't exist
                let mut meeting = Meeting::new("Reunión".to_string(), None);
                meeting.id = meeting_id.clone();
                meeting.audio_path = Some(resolved_path.clone());
                meeting.state = MeetingState::Recording;
                let _ = repo.create(meeting).await;
            }
        }
    }

    // Publicar la sesión (fuente de verdad para el tray y para rechazar
    // solapamientos).
    *slot = Some(RecordingSession {
        meeting_id,
        audio_path: resolved_path.clone(),
        source,
        recorder,
    });

    Ok(resolved_path)
}

pub async fn stop_recording(meeting_id: String) -> Result<(), String> {
    // Sacar la sesión del lock antes de esperar al grabador (que puede tardar
    // segundos en audio de sistema), para no serializar otras operaciones.
    let session = {
        let mut slot = session_slot().lock().await;
        match slot.as_ref() {
            None => return Ok(()),
            Some(s) if s.meeting_id != meeting_id => {
                return Err("La grabación activa no corresponde a esta reunión".to_string());
            }
            Some(_) => slot.take().expect("checked Some above"),
        }
    };

    let stop_result = session.recorder.stop_recording().await;
    drop(session);

    match &stop_result {
        Ok(()) => tracing::info!(meeting_id = %meeting_id, "grabación detenida"),
        Err(e) => {
            tracing::error!(meeting_id = %meeting_id, error = %e, "fallo al detener la grabación")
        }
    }

    // La grabación terminó; el audio está en disco pero el pipeline
    // (diarización/transcripción/resumen) aún no corrió. Si el stop falló,
    // marcamos error para no dejar la reunión "grabando" para siempre.
    if let Ok(repo) = get_database() {
        if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
            meeting.state = match &stop_result {
                Ok(()) => MeetingState::Processing,
                Err(e) => MeetingState::Error(format!("Error al detener la grabación: {}", e)),
            };
            let _ = repo.update(meeting).await;
        }
    }

    stop_result.map_err(|e| e.to_string())
}

pub async fn pause_recording() -> Result<(), String> {
    let slot = session_slot().lock().await;

    if let Some(session) = slot.as_ref() {
        session
            .recorder
            .pause_recording()
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Al arrancar la app no puede haber ninguna grabación activa. Si quedó alguna
/// reunión en estado `recording` (p. ej. la app se cerró o crasheó a mitad de
/// una grabación), la marca como error para que no aparezca "grabando" para
/// siempre en la interfaz.
pub async fn reconcile_interrupted_recordings() {
    let Ok(repo) = get_database() else {
        return;
    };
    let Ok(meetings) = repo.list().await else {
        return;
    };
    for mut meeting in meetings {
        if meeting.state == MeetingState::Recording {
            meeting.state = MeetingState::Error(
                "La grabación se interrumpió al cerrarse la aplicación.".to_string(),
            );
            let _ = repo.update(meeting).await;
        }
    }
}

pub fn list_audio_devices() -> Result<Vec<services::devices::AudioDevice>, String> {
    list_input_devices().map_err(|e| e.to_string())
}

pub async fn diarize_audio(
    audio_path: String,
    num_speakers: Option<i32>,
) -> Result<Vec<Segment>, String> {
    let engine = SherpaDiarizationEngine::new();
    engine
        .diarize(&audio_path, num_speakers)
        .await
        .map_err(|e| e.to_string())
}

pub async fn transcribe_audio(
    audio_path: String,
    model: String,
) -> Result<TranscriptionOutput, String> {
    // Obtener API key del keychain
    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    // El endpoint lo decide el backend (config), no el frontend.
    let endpoint = ConfigManager::new().transcription_endpoint();
    let transcriber = CloudTranscriber::new(endpoint, api_key, model);
    transcriber
        .transcribe_full(&audio_path)
        .await
        .map_err(|e| e.to_string())
}

/// Cruza la diarización con la transcripción con marcas de tiempo para obtener
/// la vista de "quién dijo qué". Es puro (sin I/O ni red), así que no necesita
/// ser async ni devolver Result.
pub fn attribute_speakers_to_transcript(
    diarization: Vec<Segment>,
    transcript: Vec<TranscriptSegment>,
) -> Vec<SpeakerUtterance> {
    attribute_speakers(&diarization, &transcript)
}

pub async fn summarize_text(
    text: String,
    model: String,
    instructions: Option<String>,
) -> Result<String, String> {
    // Obtener API key del keychain
    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let endpoint = ConfigManager::new().chat_endpoint();
    let summarizer = LlmSummarizer::new(endpoint, api_key, model);
    summarizer
        .summarize_with_instructions(&text, instructions.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Persiste diarización + transcripción tan pronto están listas — deliberadamente
/// separado de guardar el resumen (ver `update_summary`). Transcribir una
/// reunión larga es la parte lenta y costosa (llamada de red a Whisper); si el
/// paso de resumen que viene después falla (red, gateway caído), esta llamada ya
/// dejó la transcripción a salvo en disco, así que retomar solo cuesta generar
/// el resumen de nuevo en vez de re-transcribir todo el audio.
pub async fn save_transcript_results(
    meeting_id: String,
    segments: Vec<Segment>,
    utterances: Vec<SpeakerUtterance>,
    transcript: String,
) -> Result<(), String> {
    let repo = get_database()?;

    repo.save_segments(&meeting_id, &segments)
        .map_err(|e| e.to_string())?;

    repo.save_utterances(&meeting_id, &utterances)
        .map_err(|e| e.to_string())?;

    repo.save_transcript(&meeting_id, &transcript)
        .map_err(|e| e.to_string())?;

    // Indexar la transcripción para la búsqueda semántica cross-meeting del
    // asistente global. Solo depende del transcript, no del resumen, así que
    // va aquí y no queda condicionado a que el resumen se genere con éxito.
    // No bloqueante en caso de error — sin API key configurada, o si el
    // gateway de embeddings falla, la reunión sigue siendo completamente
    // usable, solo no aparecerá en "preguntar a mimo" hasta que se reprocese.
    if let Ok(Some(api_key)) = KeychainManager::get_api_key() {
        let config = ConfigManager::new();
        let chunks = services::chunk_text(&transcript, 800);
        if !chunks.is_empty() {
            let embed_client =
                services::EmbeddingClient::new(config.embeddings_endpoint(), api_key);
            if let Ok(vectors) = embed_client.embed(&chunks).await {
                if vectors.len() == chunks.len() {
                    let pairs: Vec<(String, Vec<f32>)> = chunks.into_iter().zip(vectors).collect();
                    let _ = repo.save_chunks(&meeting_id, &pairs);
                }
            }
        }
    }

    Ok(())
}

/// Marca una reunión como fallida cuando el pipeline (diarización,
/// transcripción o resumen) aborta con error, para que deje de mostrarse
/// como "grabando"/"procesando" indefinidamente en la interfaz.
pub async fn set_expected_speakers(
    meeting_id: String,
    expected_speakers: Option<i32>,
) -> Result<(), String> {
    let repo = get_database()?;
    repo.set_expected_speakers(&meeting_id, expected_speakers)
        .map_err(|e| e.to_string())
}

pub async fn mark_meeting_error(meeting_id: String, error: String) -> Result<(), String> {
    let repo = get_database()?;
    if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
        meeting.state = MeetingState::Error(error);
        let _ = repo.update(meeting).await;
    }
    Ok(())
}

/// Borra una reunión por completo: el archivo de audio en disco (si existe)
/// y todos sus datos asociados en la BD (segmentos, transcripción, resumen,
/// historial de chat, y finalmente la propia reunión). Antes solo se borraba
/// la fila de `meetings`, dejando el WAV huérfano en disco y las demás
/// tablas con filas sin dueño.
pub async fn delete_meeting(meeting_id: String) -> Result<(), String> {
    let repo = get_database()?;

    if let Ok(Some(meeting)) = repo.get(&meeting_id).await {
        if let Some(audio_path) = meeting.audio_path {
            // No es un error si el archivo ya no existe; sí lo registramos
            // pero no bloqueamos el borrado del registro por esto.
            let _ = std::fs::remove_file(&audio_path);
        }
    }

    repo.delete(&meeting_id).await.map_err(|e| e.to_string())
}

pub async fn get_meeting_data(meeting_id: String) -> Result<MeetingData, String> {
    let repo = get_database()?;

    let segments = repo.get_segments(&meeting_id).map_err(|e| e.to_string())?;

    let utterances = repo
        .get_utterances(&meeting_id)
        .map_err(|e| e.to_string())?;

    let transcript = repo
        .get_transcript(&meeting_id)
        .map_err(|e| e.to_string())?;

    let summary = repo.get_summary(&meeting_id).map_err(|e| e.to_string())?;

    Ok(MeetingData {
        segments,
        utterances,
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
    pub utterances: Vec<SpeakerUtterance>,
    pub transcript: Option<String>,
    pub summary: Option<String>,
}

pub async fn get_chat_history(meeting_id: String) -> Result<Vec<services::ChatMessage>, String> {
    let repo = get_database()?;
    repo.get_chat_history(&meeting_id)
        .map_err(|e| e.to_string())
}

/// Responde una pregunta sobre la reunión usando su transcripción completa
/// (y resumen, si existe) como contexto, más el historial previo de la
/// conversación. Persiste tanto la pregunta como la respuesta antes de
/// devolver esta última, para que el historial sobreviva a cerrar la app.
pub async fn ask_meeting_question(
    meeting_id: String,
    question: String,
    model: String,
) -> Result<String, String> {
    let repo = get_database()?;

    let transcript = repo
        .get_transcript(&meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "Esta reunión aún no tiene transcripción — ejecuta el pipeline primero.".to_string()
        })?;

    let summary = repo.get_summary(&meeting_id).map_err(|e| e.to_string())?;
    let history = repo
        .get_chat_history(&meeting_id)
        .map_err(|e| e.to_string())?;

    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let endpoint = ConfigManager::new().chat_endpoint();
    let assistant = services::MeetingChatAssistant::new(endpoint, api_key, model);
    let answer = assistant
        .ask(&transcript, summary.as_deref(), &history, &question)
        .await
        .map_err(|e| e.to_string())?;

    // Persistir ambos turnos solo tras obtener respuesta exitosa, para no
    // dejar una pregunta huérfana sin respuesta si la llamada falla.
    repo.save_chat_message(&meeting_id, "user", &question)
        .map_err(|e| e.to_string())?;
    repo.save_chat_message(&meeting_id, "assistant", &answer)
        .map_err(|e| e.to_string())?;

    Ok(answer)
}

#[derive(Serialize, Deserialize)]
pub struct GlobalSource {
    pub meeting_id: String,
    pub meeting_title: String,
}

#[derive(Serialize, Deserialize)]
pub struct GlobalAnswer {
    pub answer: String,
    pub sources: Vec<GlobalSource>,
}

pub async fn get_global_chat_history(
    conversation_id: String,
) -> Result<Vec<services::ChatMessage>, String> {
    let repo = get_database()?;
    repo.get_global_chat_history(&conversation_id)
        .map_err(|e| e.to_string())
}

pub async fn list_global_conversations() -> Result<Vec<services::Conversation>, String> {
    let repo = get_database()?;
    repo.list_conversations().map_err(|e| e.to_string())
}

/// Crea una conversación nueva del asistente global. Nace con un título
/// placeholder que se reemplaza por la primera pregunta del usuario (ver
/// `ask_global_question`); también puede renombrarse a mano.
pub async fn create_global_conversation() -> Result<services::Conversation, String> {
    let repo = get_database()?;
    let conv = services::Conversation {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Nuevo chat".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    repo.create_conversation(&conv).map_err(|e| e.to_string())?;
    Ok(conv)
}

pub async fn rename_global_conversation(
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let repo = get_database()?;
    repo.rename_conversation(&conversation_id, &title)
        .map_err(|e| e.to_string())
}

pub async fn delete_global_conversation(conversation_id: String) -> Result<(), String> {
    let repo = get_database()?;
    repo.delete_conversation(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Sobrescribe el resumen de una reunión (edición manual, regeneración, o el
/// guardado final del pipeline) y la marca como completada — tener resumen
/// guardado ES la definición de "completada" en esta app, así que este es el
/// único lugar que hace esa transición de estado.
pub async fn update_summary(meeting_id: String, summary: String) -> Result<(), String> {
    let repo = get_database()?;
    repo.save_summary(&meeting_id, &summary)
        .map_err(|e| e.to_string())?;

    if let Ok(Some(mut meeting)) = repo.get(&meeting_id).await {
        meeting.state = MeetingState::Completed;
        let _ = repo.update(meeting).await;
    }

    Ok(())
}

pub async fn update_meeting_title(meeting_id: String, title: String) -> Result<(), String> {
    let repo = get_database()?;
    repo.set_meeting_title(&meeting_id, &title)
        .map_err(|e| e.to_string())
}

pub async fn update_meeting_category(
    meeting_id: String,
    category: Option<String>,
) -> Result<(), String> {
    let repo = get_database()?;
    repo.set_meeting_category(&meeting_id, category.as_deref())
        .map_err(|e| e.to_string())
}

/// Genera (con IA) un título para la reunión a partir de su resumen, o del
/// transcript si aún no hay resumen, y lo persiste. Devuelve el título para que
/// el frontend lo muestre sin recargar.
pub async fn generate_meeting_title(meeting_id: String, model: String) -> Result<String, String> {
    let repo = get_database()?;

    let source = match repo.get_summary(&meeting_id).map_err(|e| e.to_string())? {
        Some(s) => s,
        None => repo
            .get_transcript(&meeting_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "La reunión aún no tiene resumen ni transcripción.".to_string())?,
    };

    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let endpoint = ConfigManager::new().chat_endpoint();
    let summarizer = LlmSummarizer::new(endpoint, api_key, model);
    let title = summarizer
        .generate_title(&source)
        .await
        .map_err(|e| e.to_string())?;

    repo.set_meeting_title(&meeting_id, &title)
        .map_err(|e| e.to_string())?;

    Ok(title)
}

/// Recupera los fragmentos más relevantes para `query` de entre TODAS las
/// reuniones indexadas: similitud coseno en memoria para generar candidatos
/// (rápido, sin llamada de red) y luego rerank real sobre esos candidatos
/// para la selección final — la combinación estándar de "candidato barato +
/// reordenamiento preciso" en vez de mandar cientos de fragmentos al rerank.
async fn find_relevant_sources(
    repo: &MeetingRepositoryImpl,
    api_key: &str,
    query: &str,
) -> Result<Vec<services::SourceChunk>, String> {
    let config = ConfigManager::new();

    let all_chunks = repo.get_all_chunks().map_err(|e| e.to_string())?;
    if all_chunks.is_empty() {
        return Ok(vec![]);
    }

    let embed_client =
        services::EmbeddingClient::new(config.embeddings_endpoint(), api_key.to_string());
    let query_embedding = embed_client
        .embed(&[query.to_string()])
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "Respuesta de embedding vacía".to_string())?;

    let mut scored: Vec<(f32, &services::ChunkRow)> = all_chunks
        .iter()
        .map(|c| {
            (
                services::cosine_similarity(&query_embedding, &c.embedding),
                c,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.truncate(20);

    let documents: Vec<String> = scored.iter().map(|(_, c)| c.text.clone()).collect();
    let rerank_client = services::RerankClient::new(config.rerank_endpoint(), api_key.to_string());
    let reranked = rerank_client
        .rerank(query, &documents)
        .await
        .map_err(|e| e.to_string())?;

    let mut sources = Vec::new();
    for r in reranked.into_iter().take(6) {
        if let Some((_, chunk)) = scored.get(r.index) {
            let title = repo
                .get(&chunk.meeting_id)
                .await
                .ok()
                .flatten()
                .map(|m| m.title)
                .unwrap_or_else(|| "Reunión".to_string());
            sources.push(services::SourceChunk {
                meeting_id: chunk.meeting_id.clone(),
                meeting_title: title,
                text: chunk.text.clone(),
            });
        }
    }

    Ok(sources)
}

/// Responde preguntas sobre el historial COMPLETO de reuniones, a
/// diferencia de `ask_meeting_question` que solo conoce una. En vez de
/// meter todas las transcripciones en el contexto (inviable a partir de
/// unas pocas reuniones), recupera solo los fragmentos relevantes vía
/// embedding + rerank y los cita en la respuesta para que el usuario pueda
/// abrir la reunión correspondiente.
pub async fn ask_global_question(
    conversation_id: String,
    question: String,
    model: String,
) -> Result<GlobalAnswer, String> {
    let repo = get_database()?;

    let api_key = KeychainManager::get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or_else(|| "API key not configured".to_string())?;

    let sources = find_relevant_sources(&repo, &api_key, &question).await?;
    if sources.is_empty() {
        return Err(
            "Todavía no hay reuniones indexadas para buscar. Procesa al menos una reunión con el pipeline completo primero."
                .to_string(),
        );
    }

    let history = repo
        .get_global_chat_history(&conversation_id)
        .map_err(|e| e.to_string())?;

    let endpoint = ConfigManager::new().chat_endpoint();
    let assistant = services::GlobalAssistant::new(endpoint, api_key, model);
    let answer = assistant
        .ask(&sources, &history, &question)
        .await
        .map_err(|e| e.to_string())?;

    repo.save_global_chat_message(&conversation_id, "user", &question)
        .map_err(|e| e.to_string())?;
    repo.save_global_chat_message(&conversation_id, "assistant", &answer)
        .map_err(|e| e.to_string())?;

    // Auto-nombrar la conversación con la primera pregunta si sigue con el
    // título placeholder — así la lista de chats es reconocible sin que el
    // usuario tenga que renombrar a mano.
    if let Ok(Some(title)) = repo.get_conversation_title(&conversation_id) {
        if title == "Nuevo chat" {
            let short: String = question.chars().take(60).collect();
            let _ = repo.rename_conversation(&conversation_id, short.trim());
        }
    }

    // Deduplicar por reunión: si varios fragmentos relevantes vinieron de la
    // misma reunión, se cita una sola vez.
    let mut seen = std::collections::HashSet::new();
    let mut dedup_sources = Vec::new();
    for s in &sources {
        if seen.insert(s.meeting_id.clone()) {
            dedup_sources.push(GlobalSource {
                meeting_id: s.meeting_id.clone(),
                meeting_title: s.meeting_title.clone(),
            });
        }
    }

    Ok(GlobalAnswer {
        answer,
        sources: dedup_sources,
    })
}

#[cfg(test)]
mod manual_chat_verification {
    use super::*;

    // Test manual, ignorado por defecto: pregunta algo real sobre una
    // reunión real ya existente en la BD del usuario. Requiere red, API key
    // en Keychain, y que MEETING_ID exista con transcripción guardada.
    //   MEETING_ID=<id> cargo test --lib manual_chat_verification -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn ask_real_question_about_real_meeting() {
        let meeting_id =
            std::env::var("MEETING_ID").expect("set MEETING_ID to an existing meeting id");

        let answer = ask_meeting_question(
            meeting_id.clone(),
            "¿De qué trató esta reunión, en una frase?".to_string(),
            "mimo-v2.5".to_string(),
        )
        .await
        .expect("ask_meeting_question failed");

        println!("Respuesta: {}", answer);
        assert!(!answer.trim().is_empty());

        let history = get_chat_history(meeting_id)
            .await
            .expect("get_chat_history failed");
        println!("Historial guardado: {} mensajes", history.len());
        assert!(
            history.len() >= 2,
            "debería haber al menos pregunta+respuesta"
        );
    }
}

#[cfg(test)]
mod manual_delete_verification {
    use super::*;
    use domain::MeetingRepository;

    #[tokio::test]
    #[ignore]
    async fn delete_cascade_removes_everything() {
        let repo = get_database().expect("db init failed");

        // Crear reunión de prueba con un WAV falso.
        let fake_wav = std::env::temp_dir().join("resomer_delete_test.wav");
        std::fs::write(&fake_wav, b"fake wav content").unwrap();

        let mut meeting = Meeting::new("DELETE_TEST".to_string(), None);
        meeting.audio_path = Some(fake_wav.to_str().unwrap().to_string());
        let meeting = repo.create(meeting).await.expect("create failed");

        repo.save_segments(
            &meeting.id,
            &[Segment {
                start: 0.0,
                end: 1.0,
                speaker: "Speaker-0".into(),
            }],
        )
        .unwrap();
        repo.save_transcript(&meeting.id, "texto de prueba")
            .unwrap();
        repo.save_summary(&meeting.id, "resumen de prueba").unwrap();
        repo.save_chat_message(&meeting.id, "user", "pregunta de prueba")
            .unwrap();

        assert!(
            fake_wav.exists(),
            "el WAV falso debería existir antes de borrar"
        );
        assert!(repo.get(&meeting.id).await.unwrap().is_some());
        assert!(!repo.get_segments(&meeting.id).unwrap().is_empty());
        assert!(repo.get_transcript(&meeting.id).unwrap().is_some());
        assert!(repo.get_summary(&meeting.id).unwrap().is_some());
        assert!(!repo.get_chat_history(&meeting.id).unwrap().is_empty());

        // Borrar vía el comando completo (incluye borrado del archivo).
        delete_meeting(meeting.id.clone())
            .await
            .expect("delete failed");

        assert!(
            !fake_wav.exists(),
            "el WAV debería haberse borrado del disco"
        );
        assert!(
            repo.get(&meeting.id).await.unwrap().is_none(),
            "la reunión debería haberse borrado"
        );
        assert!(
            repo.get_segments(&meeting.id).unwrap().is_empty(),
            "segments huérfanos"
        );
        assert!(
            repo.get_transcript(&meeting.id).unwrap().is_none(),
            "transcript huérfano"
        );
        assert!(
            repo.get_summary(&meeting.id).unwrap().is_none(),
            "summary huérfano"
        );
        assert!(
            repo.get_chat_history(&meeting.id).unwrap().is_empty(),
            "chat huérfano"
        );

        println!("Borrado en cascada verificado: audio + todas las tablas asociadas.");
    }
}

#[cfg(test)]
mod api_key_mask_tests {
    use super::mask_api_key;

    #[test]
    fn masks_long_key_preserving_prefix_and_suffix() {
        let masked = mask_api_key("sk-1234567890");
        assert!(masked.starts_with("sk-"));
        assert!(masked.ends_with("890"));
        assert!(masked.contains('•'));
        assert!(!masked.contains("1234"));
    }

    #[test]
    fn masks_short_key_without_panicking() {
        assert_eq!(mask_api_key("abc"), "•••");
    }
}
