pub mod history_database;
pub mod history_repository;
pub mod manual_flow;
mod notifications;
pub mod recorder;
pub mod secret_store;
pub mod shortcut;
pub mod transcription;
pub mod tray;

use std::io;
use std::{
    sync::Arc,
};

use history_database::HistoryDatabase;
use history_repository::{
    ClearHistoryResult, DeleteTranscriptionResult, HistoryRepository, HistoryTranscription,
    HistoryTranscriptionSummary,
};
use manual_flow::{
    local_audio_file_exists, ManualTranscriptionFlow, ManualTranscriptionSettings,
    RunCompletedRecordingTranscriptionResult,
};
use recorder::{
    ActiveRecordingSession, CancelledRecording, RecorderService, RecorderSnapshot,
    RecordingInputDevice, StoppedRecording,
};
use secret_store::SecretStoreService;
use serde::{Deserialize, Serialize};
use shortcut::{RecordingShortcutStatus, ShortcutService};
use tauri::Manager;
use tauri::{AppHandle, State, WindowEvent};
use tauri_plugin_store::StoreExt;
use transcription::{
    AudioInput, GeminiProvider, Transcript, TranscriptionOptions, TranscriptionService,
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
    app: AppHandle,
    device_name: Option<String>,
    recorder_service: State<'_, RecorderService>,
) -> Result<ActiveRecordingSession, String> {
    let result = recorder_service
        .start(device_name)
        .map_err(|error| format!("failed to start recording: {error}"));
    let _ = tray::sync_recording_menu(&app);

    result
}

#[tauri::command]
fn get_recording_status(
    app: AppHandle,
    recorder_service: State<'_, RecorderService>,
) -> Result<RecorderSnapshot, String> {
    let result = recorder_service
        .snapshot()
        .map_err(|error| format!("failed to read recording status: {error}"));

    if let Ok(snapshot) = &result {
        let _ = tray::sync_recording_menu_for_snapshot(&app, snapshot);
    } else {
        let _ = tray::sync_recording_menu(&app);
    }

    result
}

#[tauri::command]
fn stop_recording(
    app: AppHandle,
    recorder_service: State<'_, RecorderService>,
) -> Result<StoppedRecording, String> {
    let result = recorder_service
        .stop()
        .map_err(|error| format!("failed to stop recording: {error}"));
    let _ = tray::sync_recording_menu(&app);

    result
}

#[tauri::command]
fn cancel_recording(
    app: AppHandle,
    recorder_service: State<'_, RecorderService>,
) -> Result<CancelledRecording, String> {
    let result = recorder_service
        .cancel()
        .map_err(|error| format!("failed to cancel recording: {error}"));
    let _ = tray::sync_recording_menu(&app);

    result
}

#[tauri::command]
fn get_recording_shortcut_status(
    shortcut_service: State<'_, ShortcutService>,
) -> Result<RecordingShortcutStatus, String> {
    shortcut_service.status()
}

#[tauri::command]
fn apply_recording_shortcut(
    app: AppHandle,
    shortcut: Option<String>,
    shortcut_service: State<'_, ShortcutService>,
) -> Result<RecordingShortcutStatus, String> {
    shortcut::apply_recording_shortcut(&app, shortcut_service.inner(), shortcut)
}

#[tauri::command]
fn save_gemini_api_key(
    api_key: String,
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<(), String> {
    secret_store_service
        .save_gemini_api_key(&api_key)
        .map_err(|error| format!("failed to save Gemini API key: {error}"))
}

#[tauri::command]
fn has_gemini_api_key(secret_store_service: State<'_, SecretStoreService>) -> Result<bool, String> {
    secret_store_service
        .has_gemini_api_key()
        .map_err(|error| format!("failed to check Gemini API key: {error}"))
}

#[tauri::command]
fn clear_gemini_api_key(
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<bool, String> {
    secret_store_service
        .clear_gemini_api_key()
        .map_err(|error| format!("failed to clear Gemini API key: {error}"))
}

#[derive(Clone, Debug)]
struct TranscriptionSettings {
    auto_copy: bool,
    save_transcription_history: bool,
    save_audio_files: bool,
    default_language: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunGeminiTranscriptionRequest {
    audio_input: AudioInput,
    #[serde(default)]
    options: TranscriptionOptions,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunCompletedRecordingTranscriptionRequest {
    audio_input: AudioInput,
}

#[tauri::command]
async fn run_gemini_transcription(
    request: RunGeminiTranscriptionRequest,
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<Transcript, String> {
    TranscriptionService::new(Arc::new(GeminiProvider::new(
        secret_store_service.inner().clone(),
    )))
    .transcribe(request.audio_input, request.options)
    .await
    .map_err(|error| format!("Gemini transcription failed: {error}"))
}

#[tauri::command]
async fn run_completed_recording_transcription(
    app: AppHandle,
    request: RunCompletedRecordingTranscriptionRequest,
    history_database: tauri::State<'_, HistoryDatabase>,
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<RunCompletedRecordingTranscriptionResult, String> {
    let settings = load_transcription_settings(&app)?;
    let manual_flow = ManualTranscriptionFlow::new(
        TranscriptionService::new(Arc::new(GeminiProvider::new(
            secret_store_service.inner().clone(),
        ))),
        HistoryRepository::new(history_database.inner().clone()),
        app.clone(),
    );

    match manual_flow
        .run(
            request.audio_input,
            ManualTranscriptionSettings {
                default_language: settings.default_language,
                auto_copy: settings.auto_copy,
                save_audio_files: settings.save_audio_files,
                save_transcription_history: settings.save_transcription_history,
            },
        )
        .await
    {
        Ok(result) => {
            notifications::notify_manual_transcription_completed(&app);
            Ok(result)
        }
        Err(error) => {
            notifications::notify_manual_transcription_failed(&app);
            Err(error)
        }
    }
}

#[tauri::command]
fn has_completed_recording_audio(request: RunCompletedRecordingTranscriptionRequest) -> bool {
    local_audio_file_exists(&request.audio_input.path)
}

fn load_transcription_settings(app: &AppHandle) -> Result<TranscriptionSettings, String> {
    let store = app
        .store("settings.json")
        .map_err(|error| format!("failed to open settings store: {error}"))?;

    let auto_copy = store
        .get("auto_copy")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let save_audio_files = store
        .get("save_audio_files")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let save_transcription_history = store
        .get("save_transcription_history")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let default_language = store
        .get("default_language")
        .and_then(|value| value.as_str().map(str::trim).map(ToOwned::to_owned))
        .filter(|value| !value.is_empty() && value != "auto");

    Ok(TranscriptionSettings {
        auto_copy,
        save_transcription_history,
        save_audio_files,
        default_language,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if tray::should_hide_on_close(window.label(), tray::is_quitting(window)) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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
            app.manage(SecretStoreService::new());
            app.manage(tray::AppExitState::default());
            app.manage(ShortcutService::default());
            tray::initialize(app.handle()).map_err(io::Error::other)?;
            shortcut::initialize(app.handle()).map_err(io::Error::other)?;

            Ok(())
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            get_history,
            get_transcription,
            delete_transcription,
            clear_history,
            list_recording_input_devices,
            start_recording,
            get_recording_status,
            stop_recording,
            cancel_recording,
            get_recording_shortcut_status,
            apply_recording_shortcut,
            save_gemini_api_key,
            has_gemini_api_key,
            clear_gemini_api_key,
            run_gemini_transcription,
            run_completed_recording_transcription,
            has_completed_recording_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
