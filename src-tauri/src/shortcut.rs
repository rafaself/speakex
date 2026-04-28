use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};
use tauri_plugin_store::StoreExt;

use crate::tray;

const SETTINGS_STORE_PATH: &str = "settings.json";
const SHORTCUT_KEY: &str = "shortcut";
const DEFAULT_RECORDING_SHORTCUT: &str = "Ctrl+Alt+A";

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecordingShortcutState {
    #[default]
    Unconfigured,
    Active,
    Invalid,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecordingShortcutSource {
    #[default]
    None,
    Saved,
    Default,
    Custom,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingShortcutStatus {
    pub state: RecordingShortcutState,
    pub source: RecordingShortcutSource,
    pub requested_shortcut: Option<String>,
    pub active_shortcut: Option<String>,
    pub detail: Option<String>,
}

#[derive(Default)]
pub struct ShortcutService {
    status: Mutex<RecordingShortcutStatus>,
}

impl ShortcutService {
    pub fn status(&self) -> Result<RecordingShortcutStatus, String> {
        self.status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "recording shortcut state is unavailable".to_string())
    }

    fn replace_status(
        &self,
        next_status: RecordingShortcutStatus,
    ) -> Result<RecordingShortcutStatus, String> {
        let mut status = self
            .status
            .lock()
            .map_err(|_| "recording shortcut state is unavailable".to_string())?;
        *status = next_status.clone();

        Ok(next_status)
    }
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(shortcut_service) = app.try_state::<ShortcutService>() else {
        return Err("recording shortcut service is unavailable during startup".to_string());
    };

    match load_saved_recording_shortcut(app) {
        Ok(saved_shortcut) => {
            let request = startup_shortcut_request(saved_shortcut);
            let _ = apply_recording_shortcut_with_source(
                app,
                shortcut_service.inner(),
                request.shortcut,
                request.source,
            )?;
        }
        Err(error) => {
            let _ = shortcut_service.replace_status(unavailable_status(
                RecordingShortcutSource::None,
                None,
                format!("failed to load recording shortcut setting: {error}"),
            ))?;
        }
    }

    Ok(())
}

pub fn apply_recording_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    shortcut_service: &ShortcutService,
    shortcut: Option<String>,
) -> Result<RecordingShortcutStatus, String> {
    apply_recording_shortcut_with_source(
        app,
        shortcut_service,
        shortcut,
        RecordingShortcutSource::Custom,
    )
}

fn apply_recording_shortcut_with_source<R: Runtime>(
    app: &AppHandle<R>,
    shortcut_service: &ShortcutService,
    shortcut: Option<String>,
    source: RecordingShortcutSource,
) -> Result<RecordingShortcutStatus, String> {
    let requested_shortcut = normalize_shortcut_value(shortcut.as_deref());
    let parsed_shortcut = requested_shortcut
        .as_deref()
        .map(parse_shortcut)
        .transpose();

    if let Ok(Some(parsed_shortcut)) = &parsed_shortcut {
        let canonical_shortcut = parsed_shortcut.into_string();
        let current_status = shortcut_service.status()?;

        if current_status.state == RecordingShortcutState::Active
            && current_status.active_shortcut.as_deref() == Some(canonical_shortcut.as_str())
        {
            return shortcut_service.replace_status(active_status(source, canonical_shortcut));
        }
    }

    unregister_active_shortcut(app, shortcut_service)?;

    match parsed_shortcut {
        Ok(Some(parsed_shortcut)) => {
            let canonical_shortcut = parsed_shortcut.into_string();

            match register_recording_shortcut(app, canonical_shortcut.as_str()) {
                Ok(()) => {
                    shortcut_service.replace_status(active_status(source, canonical_shortcut))
                }
                Err(error) => shortcut_service.replace_status(unavailable_status(
                    source,
                    Some(canonical_shortcut),
                    format!("recording shortcut is unavailable: {error}"),
                )),
            }
        }
        Ok(None) => shortcut_service.replace_status(RecordingShortcutStatus::default()),
        Err(error) => shortcut_service.replace_status(invalid_status(
            source,
            requested_shortcut,
            format!("recording shortcut is invalid: {error}"),
        )),
    }
}

fn register_recording_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    shortcut: &str,
) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, handle_recording_shortcut_event)
        .map_err(|error| format!("failed to register recording shortcut: {error}"))
}

fn unregister_active_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    shortcut_service: &ShortcutService,
) -> Result<(), String> {
    let active_shortcut = shortcut_service.status()?.active_shortcut;

    if let Some(active_shortcut) = active_shortcut {
        if app
            .global_shortcut()
            .is_registered(active_shortcut.as_str())
        {
            app.global_shortcut()
                .unregister(active_shortcut.as_str())
                .map_err(|error| format!("failed to unregister recording shortcut: {error}"))?;
        }
    }

    Ok(())
}

fn handle_recording_shortcut_event<R: Runtime>(
    app: &AppHandle<R>,
    _shortcut: &Shortcut,
    event: ShortcutEvent,
) {
    if event.state == ShortcutState::Pressed && tray::toggle_recording(app).is_err() {
        eprintln!("failed to toggle recording from the global shortcut");
    }
}

fn load_saved_recording_shortcut<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("failed to open settings store: {error}"))?;

    Ok(normalize_shortcut_value(
        store
            .get(SHORTCUT_KEY)
            .as_ref()
            .and_then(|value| value.as_str()),
    ))
}

fn parse_shortcut(shortcut: &str) -> Result<Shortcut, tauri_plugin_global_shortcut::Error> {
    shortcut.parse().map_err(Into::into)
}

fn normalize_shortcut_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn active_status(
    source: RecordingShortcutSource,
    active_shortcut: String,
) -> RecordingShortcutStatus {
    RecordingShortcutStatus {
        state: RecordingShortcutState::Active,
        source,
        requested_shortcut: Some(active_shortcut.clone()),
        active_shortcut: Some(active_shortcut),
        detail: None,
    }
}

fn invalid_status(
    source: RecordingShortcutSource,
    requested_shortcut: Option<String>,
    detail: String,
) -> RecordingShortcutStatus {
    RecordingShortcutStatus {
        state: RecordingShortcutState::Invalid,
        source,
        requested_shortcut,
        active_shortcut: None,
        detail: Some(detail),
    }
}

fn unavailable_status(
    source: RecordingShortcutSource,
    requested_shortcut: Option<String>,
    detail: String,
) -> RecordingShortcutStatus {
    RecordingShortcutStatus {
        state: RecordingShortcutState::Unavailable,
        source,
        requested_shortcut,
        active_shortcut: None,
        detail: Some(detail),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartupShortcutRequest {
    shortcut: Option<String>,
    source: RecordingShortcutSource,
}

fn startup_shortcut_request(saved_shortcut: Option<String>) -> StartupShortcutRequest {
    match saved_shortcut {
        Some(saved_shortcut) => StartupShortcutRequest {
            shortcut: Some(saved_shortcut),
            source: RecordingShortcutSource::Saved,
        },
        None => StartupShortcutRequest {
            shortcut: Some(DEFAULT_RECORDING_SHORTCUT.to_string()),
            source: RecordingShortcutSource::Default,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_shortcut_values() {
        assert_eq!(normalize_shortcut_value(None), None);
        assert_eq!(normalize_shortcut_value(Some("")), None);
        assert_eq!(normalize_shortcut_value(Some("  ")), None);
        assert_eq!(
            normalize_shortcut_value(Some("  Ctrl+Alt+A  ")),
            Some("Ctrl+Alt+A".to_string())
        );
    }

    #[test]
    fn prefers_saved_shortcut_at_startup() {
        assert_eq!(
            startup_shortcut_request(Some("Ctrl+Shift+R".to_string())),
            StartupShortcutRequest {
                shortcut: Some("Ctrl+Shift+R".to_string()),
                source: RecordingShortcutSource::Saved,
            }
        );
    }

    #[test]
    fn falls_back_to_default_shortcut_at_startup() {
        assert_eq!(
            startup_shortcut_request(None),
            StartupShortcutRequest {
                shortcut: Some(DEFAULT_RECORDING_SHORTCUT.to_string()),
                source: RecordingShortcutSource::Default,
            }
        );
    }

    #[test]
    fn builds_active_shortcut_status() {
        assert_eq!(
            active_status(
                RecordingShortcutSource::Default,
                DEFAULT_RECORDING_SHORTCUT.to_string(),
            ),
            RecordingShortcutStatus {
                state: RecordingShortcutState::Active,
                source: RecordingShortcutSource::Default,
                requested_shortcut: Some(DEFAULT_RECORDING_SHORTCUT.to_string()),
                active_shortcut: Some(DEFAULT_RECORDING_SHORTCUT.to_string()),
                detail: None,
            }
        );
    }

    #[test]
    fn builds_invalid_shortcut_status() {
        assert_eq!(
            invalid_status(
                RecordingShortcutSource::Custom,
                Some("???".to_string()),
                "recording shortcut is invalid: bad shortcut".to_string(),
            ),
            RecordingShortcutStatus {
                state: RecordingShortcutState::Invalid,
                source: RecordingShortcutSource::Custom,
                requested_shortcut: Some("???".to_string()),
                active_shortcut: None,
                detail: Some("recording shortcut is invalid: bad shortcut".to_string()),
            }
        );
    }

    #[test]
    fn builds_unavailable_shortcut_status() {
        assert_eq!(
            unavailable_status(
                RecordingShortcutSource::Saved,
                Some("Ctrl+Alt+A".to_string()),
                "recording shortcut is unavailable: already in use".to_string(),
            ),
            RecordingShortcutStatus {
                state: RecordingShortcutState::Unavailable,
                source: RecordingShortcutSource::Saved,
                requested_shortcut: Some("Ctrl+Alt+A".to_string()),
                active_shortcut: None,
                detail: Some("recording shortcut is unavailable: already in use".to_string()),
            }
        );
    }
}
