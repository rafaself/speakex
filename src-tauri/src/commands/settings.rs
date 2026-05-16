use crate::secret_store::SecretStoreService;
use crate::shortcut::{self, RecordingShortcutStatus, ShortcutService};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_recording_shortcut_status(
    shortcut_service: State<'_, ShortcutService>,
) -> Result<RecordingShortcutStatus, String> {
    shortcut_service.status()
}

#[tauri::command]
pub fn apply_recording_shortcut(
    app: AppHandle,
    shortcut: Option<String>,
    shortcut_service: State<'_, ShortcutService>,
) -> Result<RecordingShortcutStatus, String> {
    shortcut::apply_recording_shortcut(&app, shortcut_service.inner(), shortcut)
}

#[tauri::command]
pub fn save_gemini_api_key(
    api_key: String,
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<(), String> {
    secret_store_service
        .save_gemini_api_key(&api_key)
        .map_err(|error| format!("failed to save Gemini API key: {error}"))
}

#[tauri::command]
pub fn has_gemini_api_key(
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<bool, String> {
    secret_store_service
        .has_gemini_api_key()
        .map_err(|error| format!("failed to check Gemini API key: {error}"))
}

#[tauri::command]
pub fn clear_gemini_api_key(
    secret_store_service: State<'_, SecretStoreService>,
) -> Result<bool, String> {
    secret_store_service
        .clear_gemini_api_key()
        .map_err(|error| format!("failed to clear Gemini API key: {error}"))
}