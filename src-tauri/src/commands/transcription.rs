use crate::app_settings::load_transcription_settings;
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
                paste_after_transcription: false,
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
