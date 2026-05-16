use crate::history_database::HistoryDatabase;
use crate::history_repository::{
    ClearErrorLogsResult, ClearHistoryResult, DeleteTranscriptionResult, ErrorLogEntry,
    HistoryRepository, HistoryTranscription, HistoryTranscriptionSummary, NewErrorLog,
};
use serde::Deserialize;

#[tauri::command]
pub fn get_history(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<Vec<HistoryTranscriptionSummary>, String> {
    HistoryRepository::new(history_database.inner().clone()).get_history()
}

#[tauri::command]
pub fn get_transcription(
    id: String,
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<Option<HistoryTranscription>, String> {
    HistoryRepository::new(history_database.inner().clone()).get_transcription(&id)
}

#[tauri::command]
pub fn delete_transcription(
    id: String,
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<DeleteTranscriptionResult, String> {
    HistoryRepository::new(history_database.inner().clone()).delete_transcription(&id)
}

#[tauri::command]
pub fn clear_history(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<ClearHistoryResult, String> {
    HistoryRepository::new(history_database.inner().clone()).clear_history()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateErrorLogRequest {
    scope: String,
    source: String,
    summary: String,
    detail: String,
}

#[tauri::command]
pub fn get_error_logs(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<Vec<ErrorLogEntry>, String> {
    HistoryRepository::new(history_database.inner().clone()).get_error_logs()
}

#[tauri::command]
pub fn clear_error_logs(
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<ClearErrorLogsResult, String> {
    HistoryRepository::new(history_database.inner().clone()).clear_error_logs()
}

#[tauri::command]
pub fn create_error_log(
    request: CreateErrorLogRequest,
    history_database: tauri::State<'_, HistoryDatabase>,
) -> Result<(), String> {
    HistoryRepository::new(history_database.inner().clone()).save_error_log(&NewErrorLog {
        scope: request.scope,
        source: request.source,
        summary: request.summary,
        detail: request.detail,
    })
}