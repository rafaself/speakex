use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

pub const SETTINGS_STORE_PATH: &str = "settings.json";

const AUTO_COPY_KEY: &str = "auto_copy";
const DEFAULT_LANGUAGE_KEY: &str = "default_language";
const PASTE_AFTER_SHORTCUT_RECORDING_KEY: &str = "paste_after_shortcut_recording";
const SAVE_AUDIO_FILES_KEY: &str = "save_audio_files";
const SAVE_TRANSCRIPTION_HISTORY_KEY: &str = "save_transcription_history";
const SELECTED_MICROPHONE_KEY: &str = "selected_microphone";
const SHORTCUT_KEY: &str = "shortcut";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptionSettings {
    pub auto_copy: bool,
    pub save_transcription_history: bool,
    pub save_audio_files: bool,
    pub default_language: Option<String>,
    pub paste_after_shortcut_recording: bool,
}

pub fn load_transcription_settings<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TranscriptionSettings, String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("failed to open settings store: {error}"))?;

    let auto_copy = store
        .get(AUTO_COPY_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let save_audio_files = store
        .get(SAVE_AUDIO_FILES_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let save_transcription_history = store
        .get(SAVE_TRANSCRIPTION_HISTORY_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let paste_after_shortcut_recording = store
        .get(PASTE_AFTER_SHORTCUT_RECORDING_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let default_language = normalize_optional_string(
        store
            .get(DEFAULT_LANGUAGE_KEY)
            .as_ref()
            .and_then(|value| value.as_str()),
    )
    .filter(|value| value != "auto");

    Ok(TranscriptionSettings {
        auto_copy,
        save_transcription_history,
        save_audio_files,
        default_language,
        paste_after_shortcut_recording,
    })
}

pub fn load_selected_microphone_name<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    app.store(SETTINGS_STORE_PATH).ok().and_then(|store| {
        normalize_optional_string(
            store
                .get(SELECTED_MICROPHONE_KEY)
                .as_ref()
                .and_then(|value| value.as_str()),
        )
        .filter(|value| value != "default")
    })
}

pub fn load_recording_shortcut<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("failed to open settings store: {error}"))?;

    Ok(normalize_optional_string(
        store
            .get(SHORTCUT_KEY)
            .as_ref()
            .and_then(|value| value.as_str()),
    ))
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
