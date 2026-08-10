use resomer_backend::domain::meeting::Meeting;
use resomer_backend::domain::{Segment, SpeakerUtterance};
use resomer_backend::services::devices::AudioDevice;
use resomer_backend::services::{
    ChatMessage, Conversation, TranscriptSegment, TranscriptionOutput,
};
use resomer_backend::{
    ask_global_question as lib_ask_global_question, ask_meeting_question as lib_ask_meeting_question,
    attribute_speakers_to_transcript as lib_attribute_speakers, create_meeting as lib_create_meeting,
    delete_api_key as lib_delete_api_key, delete_meeting as lib_delete_meeting,
    diarize_audio as lib_diarize_audio, get_api_key as lib_get_api_key,
    get_chat_history as lib_get_chat_history,
    get_global_chat_history as lib_get_global_chat_history,
    get_meeting_data as lib_get_meeting_data, list_audio_devices as lib_list_audio_devices,
    list_meetings as lib_list_meetings, mark_meeting_error as lib_mark_meeting_error,
    pause_recording as lib_pause_recording, save_api_key as lib_save_api_key,
    save_transcript_results as lib_save_transcript_results,
    set_expected_speakers as lib_set_expected_speakers, start_recording as lib_start_recording,
    stop_recording as lib_stop_recording, summarize_text as lib_summarize_text,
    test_connection as lib_test_connection, transcribe_audio as lib_transcribe_audio,
    ActiveRecording, GlobalAnswer, MeetingData, TestConnectionResponse,
};
use resomer_backend::{
    create_global_conversation as lib_create_global_conversation,
    delete_global_conversation as lib_delete_global_conversation,
    generate_meeting_title as lib_generate_meeting_title,
    list_global_conversations as lib_list_global_conversations,
    rename_global_conversation as lib_rename_global_conversation,
    update_meeting_category as lib_update_meeting_category,
    update_meeting_title as lib_update_meeting_title, update_summary as lib_update_summary,
};

#[tauri::command]
fn save_api_key(api_key: String) -> Result<(), String> {
    lib_save_api_key(api_key)
}

#[tauri::command]
fn get_api_key() -> Result<Option<String>, String> {
    lib_get_api_key()
}

#[tauri::command]
fn delete_api_key() -> Result<(), String> {
    lib_delete_api_key()
}

#[tauri::command]
async fn test_connection(api_key: String) -> Result<TestConnectionResponse, String> {
    lib_test_connection(api_key).await
}

#[tauri::command]
async fn create_meeting(
    title: String,
    expected_speakers: Option<i32>,
    category: Option<String>,
) -> Result<Meeting, String> {
    lib_create_meeting(title, expected_speakers, category).await
}

#[tauri::command]
async fn start_recording(
    app: tauri::AppHandle,
    meeting_id: String,
    output_path: String,
    source: String,
) -> Result<String, String> {
    let show_meter = source == "system_audio" || source == "both";
    let result = lib_start_recording(app.clone(), meeting_id, output_path, source).await;
    if result.is_ok() {
        let _ = update_tray_recording(&app, true, show_meter);
    }
    result
}

#[tauri::command]
async fn stop_recording(app: tauri::AppHandle, meeting_id: String) -> Result<(), String> {
    let result = lib_stop_recording(meeting_id).await;
    if result.is_ok() {
        let _ = update_tray_recording(&app, false, false);
    }
    result
}

/// Estado de la grabación activa para que el frontend, al abrirse (o al
/// volver de segundo plano), refleje una grabación iniciada desde el tray.
#[tauri::command]
async fn get_active_recording() -> Option<ActiveRecording> {
    resomer_backend::get_active_recording().await
}

#[tauri::command]
async fn pause_recording() -> Result<(), String> {
    lib_pause_recording().await
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    lib_list_audio_devices()
}

#[tauri::command]
async fn diarize_audio(
    audio_path: String,
    num_speakers: Option<i32>,
) -> Result<Vec<Segment>, String> {
    lib_diarize_audio(audio_path, num_speakers).await
}

#[tauri::command]
async fn transcribe_audio(
    audio_path: String,
    api_endpoint: String,
    model: String,
) -> Result<TranscriptionOutput, String> {
    lib_transcribe_audio(audio_path, api_endpoint, model).await
}

/// Cruza diarización + transcripción con marcas de tiempo → "quién dijo qué".
#[tauri::command]
fn attribute_speakers(
    diarization: Vec<Segment>,
    transcript: Vec<TranscriptSegment>,
) -> Vec<SpeakerUtterance> {
    lib_attribute_speakers(diarization, transcript)
}

#[tauri::command]
async fn summarize_text(
    text: String,
    api_endpoint: String,
    model: String,
    instructions: Option<String>,
) -> Result<String, String> {
    lib_summarize_text(text, api_endpoint, model, instructions).await
}

#[tauri::command]
async fn save_transcript_results(
    meeting_id: String,
    segments: Vec<Segment>,
    utterances: Vec<SpeakerUtterance>,
    transcript: String,
) -> Result<(), String> {
    lib_save_transcript_results(meeting_id, segments, utterances, transcript).await
}

#[tauri::command]
async fn get_meeting_data(meeting_id: String) -> Result<MeetingData, String> {
    lib_get_meeting_data(meeting_id).await
}

#[tauri::command]
async fn list_meetings() -> Result<Vec<Meeting>, String> {
    lib_list_meetings().await
}

#[tauri::command]
async fn delete_meeting(meeting_id: String) -> Result<(), String> {
    lib_delete_meeting(meeting_id).await
}

#[tauri::command]
async fn set_expected_speakers(
    meeting_id: String,
    expected_speakers: Option<i32>,
) -> Result<(), String> {
    lib_set_expected_speakers(meeting_id, expected_speakers).await
}

#[tauri::command]
async fn mark_meeting_error(meeting_id: String, error: String) -> Result<(), String> {
    lib_mark_meeting_error(meeting_id, error).await
}

#[tauri::command]
async fn get_chat_history(meeting_id: String) -> Result<Vec<ChatMessage>, String> {
    lib_get_chat_history(meeting_id).await
}

#[tauri::command]
async fn ask_meeting_question(
    meeting_id: String,
    question: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    lib_ask_meeting_question(meeting_id, question, api_endpoint, model).await
}

#[tauri::command]
async fn get_global_chat_history(conversation_id: String) -> Result<Vec<ChatMessage>, String> {
    lib_get_global_chat_history(conversation_id).await
}

#[tauri::command]
async fn ask_global_question(
    conversation_id: String,
    question: String,
    api_endpoint: String,
    model: String,
) -> Result<GlobalAnswer, String> {
    lib_ask_global_question(conversation_id, question, api_endpoint, model).await
}

#[tauri::command]
async fn list_global_conversations() -> Result<Vec<Conversation>, String> {
    lib_list_global_conversations().await
}

#[tauri::command]
async fn create_global_conversation() -> Result<Conversation, String> {
    lib_create_global_conversation().await
}

#[tauri::command]
async fn rename_global_conversation(conversation_id: String, title: String) -> Result<(), String> {
    lib_rename_global_conversation(conversation_id, title).await
}

#[tauri::command]
async fn delete_global_conversation(conversation_id: String) -> Result<(), String> {
    lib_delete_global_conversation(conversation_id).await
}

#[tauri::command]
async fn update_summary(meeting_id: String, summary: String) -> Result<(), String> {
    lib_update_summary(meeting_id, summary).await
}

#[tauri::command]
async fn update_meeting_title(meeting_id: String, title: String) -> Result<(), String> {
    lib_update_meeting_title(meeting_id, title).await
}

#[tauri::command]
async fn update_meeting_category(meeting_id: String, category: Option<String>) -> Result<(), String> {
    lib_update_meeting_category(meeting_id, category).await
}

#[tauri::command]
async fn generate_meeting_title(
    meeting_id: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    lib_generate_meeting_title(meeting_id, api_endpoint, model).await
}

/// Cuántos caracteres de historial muestra el mini-medidor de texto del tray.
const TRAY_METER_LEN: usize = 10;

/// Caracteres de bloque Unicode (▁ vacío → █ lleno) usados para dibujar un
/// mini-VU-metro en el menú del tray sin necesidad de generar bitmaps: macOS
/// fuerza los íconos "template" de la barra a un solo color, así que un
/// waveform a color ahí no serviría; el texto del menú sí admite esto barato.
const TRAY_METER_BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

fn level_to_block(level: f32) -> char {
    let idx = (level.clamp(0.0, 1.0) * (TRAY_METER_BLOCKS.len() - 1) as f32).round() as usize;
    TRAY_METER_BLOCKS[idx.min(TRAY_METER_BLOCKS.len() - 1)]
}

/// Estado compartido del mini-medidor del tray: historial reciente de
/// sistema y micrófono (uno por fuente) más un contador para diezmar los
/// ~15 eventos/seg del helper de audio a una cadencia razonable para un menú
/// nativo. `active` indica si la grabación actual usa audio del sistema
/// (mic-only no emite estos eventos, así que el medidor no aplica).
struct TrayMeterState {
    sys: std::sync::Mutex<std::collections::VecDeque<char>>,
    mic: std::sync::Mutex<std::collections::VecDeque<char>>,
    counter: std::sync::atomic::AtomicU32,
    active: std::sync::atomic::AtomicBool,
    // Referencias a los MenuItem del medidor actualmente en el menú del
    // tray. Los actualizamos con `set_text` (mutación in-place del ítem
    // nativo) en vez de reconstruir y reemplazar el `Menu` completo en cada
    // tick: reemplazar el menú mientras el usuario lo tiene abierto (click
    // en el ícono) hace que macOS lo cierre de golpe o crashee el proceso.
    items: std::sync::Mutex<Option<(tauri::menu::MenuItem<tauri::Wry>, tauri::menu::MenuItem<tauri::Wry>)>>,
}

impl Default for TrayMeterState {
    fn default() -> Self {
        let filled = || std::collections::VecDeque::from(vec!['▁'; TRAY_METER_LEN]);
        Self {
            sys: std::sync::Mutex::new(filled()),
            mic: std::sync::Mutex::new(filled()),
            counter: std::sync::atomic::AtomicU32::new(0),
            active: std::sync::atomic::AtomicBool::new(false),
            items: std::sync::Mutex::new(None),
        }
    }
}

impl TrayMeterState {
    fn reset(&self) {
        let filled = || std::collections::VecDeque::from(vec!['▁'; TRAY_METER_LEN]);
        *self.sys.lock().unwrap() = filled();
        *self.mic.lock().unwrap() = filled();
    }

    fn push(&self, levels: resomer_backend::services::AudioLevels) -> (String, String) {
        let push_one = |deque: &std::sync::Mutex<std::collections::VecDeque<char>>, level: f32| {
            let mut d = deque.lock().unwrap();
            d.push_back(level_to_block(level));
            while d.len() > TRAY_METER_LEN {
                d.pop_front();
            }
            d.iter().collect::<String>()
        };
        (push_one(&self.sys, levels.system), push_one(&self.mic, levels.mic))
    }
}

/// Construye el menú del ícono de la barra de estado. Es dinámico: mientras
/// se graba muestra el indicador "Grabando" y la opción de detener; en reposo
/// muestra el selector de fuente y el resto de accesos rápidos. `meter`, si
/// viene, añade dos líneas de solo-lectura con el mini-VU-metro en vivo de
/// sistema y micrófono (solo aplica a grabaciones con audio del sistema).
fn build_tray_menu(
    handle: &tauri::AppHandle,
    recording: bool,
    meter: Option<(&str, &str)>,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
    use tauri::Manager;

    let open_assistant = MenuItem::with_id(
        handle,
        "open_assistant",
        "Asistente de IA",
        true,
        None::<&str>,
    )?;
    let show = MenuItem::with_id(handle, "show", "Mostrar Resomer", true, None::<&str>)?;
    let quit = MenuItem::with_id(handle, "quit", "Salir", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(handle)?;

    if recording {
        // Elemento informativo (deshabilitado) + acción de detener.
        let status = MenuItem::with_id(handle, "rec_status", "● Grabando…", false, None::<&str>)?;
        let stop = MenuItem::with_id(
            handle,
            "stop_recording",
            "Detener grabación",
            true,
            None::<&str>,
        )?;
        let sep2 = PredefinedMenuItem::separator(handle)?;

        if let Some((sys_str, mic_str)) = meter {
            let sys_item = MenuItem::with_id(
                handle,
                "meter_sys",
                format!("Sistema  {sys_str}"),
                false,
                None::<&str>,
            )?;
            let mic_item = MenuItem::with_id(
                handle,
                "meter_mic",
                format!("Mic      {mic_str}"),
                false,
                None::<&str>,
            )?;
            *handle.state::<TrayMeterState>().items.lock().unwrap() =
                Some((sys_item.clone(), mic_item.clone()));
            Menu::with_items(
                handle,
                &[
                    &status,
                    &sys_item,
                    &mic_item,
                    &stop,
                    &sep,
                    &open_assistant,
                    &sep2,
                    &show,
                    &quit,
                ],
            )
        } else {
            *handle.state::<TrayMeterState>().items.lock().unwrap() = None;
            Menu::with_items(
                handle,
                &[&status, &stop, &sep, &open_assistant, &sep2, &show, &quit],
            )
        }
    } else {
        *handle.state::<TrayMeterState>().items.lock().unwrap() = None;
        // Submenú "Iniciar grabación" con selector de fuente. Los ids llevan
        // el valor de la fuente para reusar un mismo evento con distinto payload.
        let rec_mic = MenuItem::with_id(handle, "rec:microphone", "Micrófono", true, None::<&str>)?;
        let rec_system = MenuItem::with_id(
            handle,
            "rec:system_audio",
            "Audio del sistema",
            true,
            None::<&str>,
        )?;
        let rec_both = MenuItem::with_id(handle, "rec:both", "Ambos", true, None::<&str>)?;
        let rec_submenu = Submenu::with_id_and_items(
            handle,
            "rec_submenu",
            "Iniciar grabación",
            true,
            &[&rec_mic, &rec_system, &rec_both],
        )?;
        let run_pipeline = MenuItem::with_id(
            handle,
            "run_pipeline",
            "Ejecutar pipeline",
            true,
            None::<&str>,
        )?;
        Menu::with_items(
            handle,
            &[
                &rec_submenu,
                &run_pipeline,
                &open_assistant,
                &sep,
                &show,
                &quit,
            ],
        )
    }
}

/// Construye el ícono de la barra de estado de macOS (menu bar, junto al
/// WiFi/batería). Cada opción muestra y enfoca la ventana principal y emite un
/// evento que el frontend ya sabe orquestar (crear reunión + navegar, ejecutar
/// pipeline, abrir el asistente, detener), en vez de duplicar esa lógica aquí.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::image::Image;
    use tauri::tray::TrayIconBuilder;
    use tauri::{Listener, Manager};

    app.manage(TrayMeterState::default());

    let handle = app.handle();
    let menu = build_tray_menu(handle, false, None)?;

    // Escucha los niveles de audio en vivo (mismo evento que consume el
    // frontend) para alimentar el mini-medidor de texto del menú del tray.
    // Diezmado a ~1 de cada 4 (de ~15Hz a ~4Hz): el menú nativo no necesita
    // más para verse fluido y evita reconstruirlo en exceso.
    let listener_handle = handle.clone();
    handle.listen("system-audio-level", move |event| {
        let state = listener_handle.state::<TrayMeterState>();
        if !state.active.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        if state.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 4 != 0 {
            return;
        }
        let Ok(levels) =
            serde_json::from_str::<resomer_backend::services::AudioLevels>(event.payload())
        else {
            return;
        };
        let (sys_str, mic_str) = state.push(levels);

        let handle_for_thread = listener_handle.clone();
        let _ = listener_handle.run_on_main_thread(move || {
            let state = handle_for_thread.state::<TrayMeterState>();
            let items = state.items.lock().unwrap();
            if let Some((sys_item, mic_item)) = items.as_ref() {
                let _ = sys_item.set_text(format!("Sistema  {sys_str}"));
                let _ = mic_item.set_text(format!("Mic      {mic_str}"));
            }
        });
    });

    // Ícono monocromo tratado como "template": macOS lo pinta según el tema
    // de la barra (claro/oscuro) usando su canal alfa.
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Resomer")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            use tauri::Emitter;

            // Trae la ventana al frente para que la acción sea visible.
            let focus_main = |app: &tauri::AppHandle| {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            };

            match event.id.as_ref() {
                // Iniciar una grabación DIRECTAMENTE en el backend (sin abrir la
                // app). El frontend, si está abierto, la refleja vía evento.
                id if id.starts_with("rec:") => {
                    let source = id.trim_start_matches("rec:").to_string();
                    let show_meter = source == "system_audio" || source == "both";
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        match resomer_backend::tray_start_recording(app.clone(), source).await {
                            Ok(active) => {
                                let _ = update_tray_recording(&app, true, show_meter);
                                let _ = app.emit("recording-started", active);
                            }
                            Err(e) => {
                                let _ = app.emit("recording-error", e);
                            }
                        }
                    });
                }
                // Detener la grabación activa desde Rust: funciona aunque el
                // webview esté suspendido (ventana oculta), que era la causa de
                // que "REC" no desapareciera y el volumen siguiera bajo.
                "stop_recording" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        match resomer_backend::tray_stop_recording().await {
                            Ok(stopped) => {
                                let _ = update_tray_recording(&app, false, false);
                                if let Some(active) = stopped {
                                    let _ = app.emit("recording-stopped", active);
                                }
                            }
                            Err(e) => {
                                let _ = app.emit("recording-error", e);
                            }
                        }
                    });
                }
                "run_pipeline" => {
                    focus_main(app);
                    let _ = app.emit("tray-run-pipeline", ());
                }
                "open_assistant" => {
                    focus_main(app);
                    let _ = app.emit("tray-open-assistant", ());
                }
                "show" => focus_main(app),
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// Refleja el estado de grabación en el ícono de la barra de estado: cambia el
/// menú (mostrar "Detener"), el tooltip y añade el texto "REC" junto al ícono
/// como indicador visible. Es la fuente única de verdad del indicador; lo
/// llaman los comandos start/stop y el propio menú del tray, nunca el frontend
/// directamente (que macOS suspende con la ventana oculta).
///
/// `show_meter` indica si esta grabación captura audio del sistema (source
/// "system_audio"/"both"): solo entonces tiene sentido mostrar el
/// mini-medidor, ya que la grabación de solo micrófono no pasa por el helper
/// de Swift y nunca emite el evento de nivel.
fn update_tray_recording(
    app: &tauri::AppHandle,
    recording: bool,
    show_meter: bool,
) -> Result<(), String> {
    use tauri::Manager;

    let meter_state = app.state::<TrayMeterState>();
    meter_state
        .active
        .store(recording && show_meter, std::sync::atomic::Ordering::Relaxed);
    if recording && show_meter {
        meter_state.reset();
    }
    let meter = (recording && show_meter).then(|| {
        (
            meter_state.sys.lock().unwrap().iter().collect::<String>(),
            meter_state.mic.lock().unwrap().iter().collect::<String>(),
        )
    });

    // Las mutaciones del ícono/menú de la barra de estado deben hacerse en el
    // hilo principal en macOS. Las llamadas llegan desde tareas async en
    // segundo plano (start/stop), así que lo despachamos explícitamente; si no,
    // en macOS los cambios (p. ej. borrar "REC") pueden no aplicarse.
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id("main-tray") else {
            return;
        };
        let meter_ref = meter.as_ref().map(|(s, m)| (s.as_str(), m.as_str()));
        if let Ok(menu) = build_tray_menu(&handle, recording, meter_ref) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(if recording {
            "Resomer — Grabando…"
        } else {
            "Resomer"
        }));
        // En macOS, limpiar el título con `None` no siempre borra el texto ya
        // pintado; usar cadena vacía sí lo elimina de forma fiable.
        let _ = tray.set_title(Some(if recording { "REC" } else { "" }));
    })
    .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            build_tray(app)?;
            Ok(())
        })
        // Cerrar la ventana la oculta en vez de terminar la app, para que el
        // ícono de la barra de estado siga disponible en segundo plano. Se
        // sale realmente desde "Salir" en el menú del ícono.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            save_api_key,
            get_api_key,
            delete_api_key,
            test_connection,
            create_meeting,
            start_recording,
            stop_recording,
            pause_recording,
            list_audio_devices,
            diarize_audio,
            transcribe_audio,
            attribute_speakers,
            summarize_text,
            save_transcript_results,
            get_meeting_data,
            list_meetings,
            delete_meeting,
            mark_meeting_error,
            set_expected_speakers,
            get_chat_history,
            ask_meeting_question,
            get_global_chat_history,
            ask_global_question,
            list_global_conversations,
            create_global_conversation,
            rename_global_conversation,
            delete_global_conversation,
            update_summary,
            update_meeting_title,
            update_meeting_category,
            generate_meeting_title,
            get_active_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
