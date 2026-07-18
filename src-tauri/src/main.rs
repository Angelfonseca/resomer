use resomer_backend::domain::meeting::Meeting;
use resomer_backend::domain::Segment;
use resomer_backend::services::devices::AudioDevice;
use resomer_backend::{
    create_meeting as lib_create_meeting, delete_api_key as lib_delete_api_key,
    diarize_audio as lib_diarize_audio, get_api_key as lib_get_api_key,
    get_meeting_data as lib_get_meeting_data, list_audio_devices as lib_list_audio_devices,
    pause_recording as lib_pause_recording, save_api_key as lib_save_api_key,
    save_pipeline_results as lib_save_pipeline_results, start_recording as lib_start_recording,
    stop_recording as lib_stop_recording, summarize_text as lib_summarize_text,
    test_connection as lib_test_connection, transcribe_audio as lib_transcribe_audio,
    MeetingData, TestConnectionResponse,
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
async fn create_meeting(title: String) -> Result<Meeting, String> {
    lib_create_meeting(title).await
}

#[tauri::command]
async fn start_recording(
    meeting_id: String,
    output_path: String,
    source: String,
) -> Result<(), String> {
    lib_start_recording(meeting_id, output_path, source).await
}

#[tauri::command]
async fn stop_recording() -> Result<(), String> {
    lib_stop_recording().await
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
async fn diarize_audio(audio_path: String) -> Result<Vec<Segment>, String> {
    lib_diarize_audio(audio_path).await
}

#[tauri::command]
async fn transcribe_audio(
    audio_path: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    lib_transcribe_audio(audio_path, api_endpoint, model).await
}

#[tauri::command]
async fn summarize_text(
    text: String,
    api_endpoint: String,
    model: String,
) -> Result<String, String> {
    lib_summarize_text(text, api_endpoint, model).await
}

#[tauri::command]
async fn save_pipeline_results(
    meeting_id: String,
    segments: Vec<Segment>,
    transcript: String,
    summary: String,
) -> Result<(), String> {
    lib_save_pipeline_results(meeting_id, segments, transcript, summary).await
}

#[tauri::command]
async fn get_meeting_data(meeting_id: String) -> Result<MeetingData, String> {
    lib_get_meeting_data(meeting_id).await
}

fn main() {
    tauri::Builder::default()
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
            summarize_text,
            save_pipeline_results,
            get_meeting_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
