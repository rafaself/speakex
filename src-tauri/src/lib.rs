pub mod history_database;
pub mod history_repository;

use std::io;

use history_database::HistoryDatabase;
use history_repository::{
    ClearHistoryResult, DeleteTranscriptionResult, HistoryRepository, HistoryTranscription,
    HistoryTranscriptionSummary,
};
use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let history_database =
                history_database::initialize(app.handle()).map_err(io::Error::other)?;

            app.manage(history_database);

            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            ping,
            get_history,
            get_transcription,
            delete_transcription,
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
