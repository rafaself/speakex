use crate::history_database::HistoryDatabase;
use crate::history_repository::HistoryRepository;
use crate::manual_flow::{
    local_audio_file_exists, ManualTranscriptionFlow, ManualTranscriptionSettings,
    RunCompletedRecordingTranscriptionResult,
};
use crate::notifications;
use crate::secret_store::SecretStoreService;
use crate::transcription::{
    AudioInput, GeminiProvider, Transcript, TranscriptionOptions, TranscriptionService,
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

#[derive(Clone, Debug)]
struct TranscriptionSettings {
    auto_copy: bool,
    save_transcription_history: bool,
    save_audio_files: bool,
    default_language: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunGeminiTranscriptionRequest {
    audio_input: AudioInput,
    #[serde(default)]
    options: TranscriptionOptions,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunCompletedRecordingTranscriptionRequest {
    audio_input: AudioInput,
}

#[tauri::command]
pub async fn run_gemini_transcription(
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
pub async fn run_completed_recording_transcription(
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
pub fn has_completed_recording_audio(request: RunCompletedRecordingTranscriptionRequest) -> bool {
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