pub mod history_database;
pub mod history_repository;
pub mod recorder;
pub mod transcription;

use std::io;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use history_database::HistoryDatabase;
use history_repository::{
    ClearHistoryResult, DeleteTranscriptionResult, HistoryRepository, HistoryTranscription,
    HistoryTranscriptionSummary, NewHistoryTranscription,
};
use recorder::{
    ActiveRecordingSession, CancelledRecording, RecorderService, RecordingInputDevice,
    StoppedRecording,
};
use serde::Serialize;
use tauri::Manager;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use transcription::{
    AudioInput, MockTranscriptionProvider, Transcript, TranscriptionOptions, TranscriptionService,
};

#[tauri::command]
fn ping() -> &'static str {
    "pong from Rust"
}

#[tauri::command]
fn get_history(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<Vec<HistoryTranscriptionSummary>, String> {
    HistoryRepository::new(history_database.inner().clone()).get_history()
}

#[tauri::command]
fn get_transcription(
    id: String,
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<Option<HistoryTranscription>, String> {
    HistoryRepository::new(history_database.inner().clone()).get_transcription(&id)
}

#[tauri::command]
fn delete_transcription(
    id: String,
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<DeleteTranscriptionResult, String> {
    HistoryRepository::new(history_database.inner().clone()).delete_transcription(&id)
}

#[tauri::command]
fn clear_history(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<ClearHistoryResult, String> {
    HistoryRepository::new(history_database.inner().clone()).clear_history()
}

#[tauri::command]
fn list_recording_input_devices(
    recorder_service: State<'_, RecorderService>,
) -> Result<Vec<RecordingInputDevice>, String> {
    recorder_service
        .list_input_devices()
        .map_err(|error| format!("failed to list recording input devices: {error}"))
}

#[tauri::command]
fn start_recording(
    device_name: Option<String>,
    recorder_service: State<'_, RecorderService>,
) -> Result<ActiveRecordingSession, String> {
    recorder_service
        .start(device_name)
        .map_err(|error| format!("failed to start recording: {error}"))
}

#[tauri::command]
fn stop_recording(
    recorder_service: State<'_, RecorderService>,
) -> Result<StoppedRecording, String> {
    recorder_service
        .stop()
        .map_err(|error| format!("failed to stop recording: {error}"))
}

#[tauri::command]
fn cancel_recording(
    recorder_service: State<'_, RecorderService>,
) -> Result<CancelledRecording, String> {
    recorder_service
        .cancel()
        .map_err(|error| format!("failed to cancel recording: {error}"))
}

#[derive(Clone, Debug)]
struct MockTranscriptionSettings {
    save_transcription_history: bool,
    default_language: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunMockTranscriptionResult {
    transcript: Transcript,
    saved_to_history: bool,
}

#[tauri::command]
async fn run_mock_transcription(
    app: AppHandle,
    history_database: tauri::State<'_, HistoryDatabase>,
    transcription_service: tauri::State<'_, TranscriptionService>,
) -> Result<RunMockTranscriptionResult, String> {
    let settings = load_mock_transcription_settings(&app)?;
    let service = transcription_service.inner().clone();
    let history_repository = HistoryRepository::new(history_database.inner().clone());
    let transcript = service
        .transcribe(
            AudioInput::new(
                PathBuf::from("mock-recording.wav"),
                "audio/wav",
                Some(18_000),
            ),
            TranscriptionOptions {
                language: settings.default_language.clone(),
                prompt: Some("Release 0.5 desktop-only mock transcription pipeline.".to_string()),
                model: None,
            },
        )
        .await
        .map_err(|error| format!("mock transcription failed: {error}"))?;

    if settings.save_transcription_history {
        history_repository.save_transcription(&NewHistoryTranscription {
            id: generate_mock_transcription_id()?,
            text: transcript.text.clone(),
            provider: transcript.provider.clone(),
            model: transcript.model.clone(),
            language: transcript.language.clone(),
            duration_ms: normalize_duration_ms(transcript.duration_ms)?,
            audio_path: None,
            audio_deleted: true,
            copied_to_clipboard: false,
            error: None,
        })?;
    }

    Ok(RunMockTranscriptionResult {
        transcript,
        saved_to_history: settings.save_transcription_history,
    })
}

fn load_mock_transcription_settings(app: &AppHandle) -> Result<MockTranscriptionSettings, String> {
    let store = app
        .store("settings.json")
        .map_err(|error| format!("failed to open settings store: {error}"))?;

    let save_transcription_history = store
        .get("save_transcription_history")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let default_language = store
        .get("default_language")
        .and_then(|value| value.as_str().map(str::trim).map(ToOwned::to_owned))
        .filter(|value| !value.is_empty() && value != "auto");

    Ok(MockTranscriptionSettings {
        save_transcription_history,
        default_language,
    })
}

fn generate_mock_transcription_id() -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to generate mock transcription identifier: {error}"))?;

    Ok(format!(
        "mock-{}-{}",
        timestamp.as_secs(),
        timestamp.subsec_nanos()
    ))
}

fn normalize_duration_ms(duration_ms: Option<u64>) -> Result<Option<i64>, String> {
    duration_ms
        .map(|value| {
            i64::try_from(value).map_err(|_| {
                format!("mock transcription duration {value}ms exceeds supported history range")
            })
        })
        .transpose()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let history_database =
                history_database::initialize(app.handle()).map_err(io::Error::other)?;

            app.manage(history_database);
            let recordings_dir = app
                .path()
                .app_cache_dir()
                .map_err(io::Error::other)?
                .join("recordings");

            app.manage(RecorderService::new(recordings_dir));
            app.manage(TranscriptionService::new(Arc::new(
                MockTranscriptionProvider::new(),
            )));

            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            ping,
            get_history,
            get_transcription,
            delete_transcription,
            clear_history,
            list_recording_input_devices,
            start_recording,
            stop_recording,
            cancel_recording,
            run_mock_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
