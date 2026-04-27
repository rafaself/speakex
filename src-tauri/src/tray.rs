use std::sync::atomic::{AtomicBool, Ordering};

use crate::recorder::{RecorderPhase, RecorderService, RecorderSnapshot, RecordingInputDevice};
use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use tauri_plugin_store::StoreExt;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "main-tray";
const SETTINGS_STORE_PATH: &str = "settings.json";
const SELECTED_MICROPHONE_KEY: &str = "selected_microphone";
const SHOW_APP_MENU_ID: &str = "tray-show-main-window";
const START_RECORDING_MENU_ID: &str = "tray-start-recording";
const STOP_RECORDING_MENU_ID: &str = "tray-stop-recording";
const CANCEL_RECORDING_MENU_ID: &str = "tray-cancel-recording";
const QUIT_APP_MENU_ID: &str = "tray-quit-app";

#[derive(Default)]
pub struct AppExitState {
    quitting: AtomicBool,
}

impl AppExitState {
    fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::Relaxed);
    }

    fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::Relaxed)
    }
}

struct RecordingTrayMenu<R: Runtime> {
    start_item: MenuItem<R>,
    stop_item: MenuItem<R>,
    cancel_item: MenuItem<R>,
}

impl<R: Runtime> RecordingTrayMenu<R> {
    fn apply_state(&self, state: RecordingTrayMenuState) -> tauri::Result<()> {
        self.start_item.set_enabled(state.start_enabled)?;
        self.stop_item.set_enabled(state.stop_enabled)?;
        self.cancel_item.set_enabled(state.cancel_enabled)?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayMenuAction {
    ShowMainWindow,
    StartRecording,
    StopRecording,
    CancelRecording,
    QuitApp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RecordingTrayMenuState {
    start_enabled: bool,
    stop_enabled: bool,
    cancel_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToggleRecordingAction {
    Start,
    Stop,
}

impl RecordingTrayMenuState {
    fn from_snapshot(snapshot: &RecorderSnapshot, preferred_input_available: bool) -> Self {
        match snapshot.phase {
            RecorderPhase::Idle => Self {
                start_enabled: preferred_input_available,
                stop_enabled: false,
                cancel_enabled: false,
            },
            RecorderPhase::Recording => Self {
                start_enabled: false,
                stop_enabled: true,
                cancel_enabled: true,
            },
            RecorderPhase::Starting | RecorderPhase::Stopping | RecorderPhase::Cancelling => Self {
                start_enabled: false,
                stop_enabled: false,
                cancel_enabled: false,
            },
        }
    }
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let start_item = MenuItemBuilder::with_id(START_RECORDING_MENU_ID, "Start Recording")
        .enabled(true)
        .build(app)?;
    let stop_item = MenuItemBuilder::with_id(STOP_RECORDING_MENU_ID, "Stop Recording")
        .enabled(false)
        .build(app)?;
    let cancel_item = MenuItemBuilder::with_id(CANCEL_RECORDING_MENU_ID, "Cancel Recording")
        .enabled(false)
        .build(app)?;
    let menu = MenuBuilder::new(app)
        .text(SHOW_APP_MENU_ID, "Show SpeakEx")
        .separator()
        .item(&start_item)
        .item(&stop_item)
        .item(&cancel_item)
        .separator()
        .text(QUIT_APP_MENU_ID, "Quit SpeakEx")
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    app.manage(RecordingTrayMenu {
        start_item,
        stop_item,
        cancel_item,
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("SpeakEx")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            let _ = sync_recording_menu(tray.app_handle());
            if should_restore_for_tray_event(&event) {
                let _ = show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    let _ = sync_recording_menu(app);

    Ok(())
}

pub fn is_quitting<R: Runtime, M: Manager<R>>(manager: &M) -> bool {
    manager
        .try_state::<AppExitState>()
        .map(|state| state.is_quitting())
        .unwrap_or(false)
}

pub fn should_hide_on_close(window_label: &str, is_quitting: bool) -> bool {
    window_label == MAIN_WINDOW_LABEL && !is_quitting
}

pub fn sync_recording_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Ok(());
    };

    let snapshot = recorder_service
        .snapshot()
        .map_err(|error| format!("failed to read recording status for tray: {error}"))?;
    sync_recording_menu_for_snapshot(app, &snapshot)
}

pub fn sync_recording_menu_for_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &RecorderSnapshot,
) -> Result<(), String> {
    let preferred_input_available = if snapshot.phase == RecorderPhase::Idle {
        let Some(recorder_service) = app.try_state::<RecorderService>() else {
            return Ok(());
        };

        preferred_input_available(app, recorder_service.inner()).unwrap_or_else(|error| {
            eprintln!("{error}");
            true
        })
    } else {
        true
    };

    apply_recording_menu_state(
        app,
        RecordingTrayMenuState::from_snapshot(snapshot, preferred_input_available),
    )
}

pub fn toggle_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for shortcut toggle".to_string());
    };

    let action = toggle_recording_action(
        recorder_service
            .snapshot()
            .map_err(|error| format!("failed to read recording status for shortcut: {error}"))?
            .phase,
    )
    .ok_or_else(|| "recording cannot be toggled while the recorder is transitioning".to_string())?;

    let result = match action {
        ToggleRecordingAction::Start => handle_start_recording(app),
        ToggleRecordingAction::Stop => handle_stop_recording(app),
    };
    let _ = sync_recording_menu(app);

    result
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::AssetNotFound("main webview window".into()))?;

    window.show()?;

    if window.is_minimized()? {
        window.unminimize()?;
    }

    window.set_focus()?;

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    match tray_menu_action(menu_id) {
        Some(TrayMenuAction::ShowMainWindow) => {
            let _ = show_main_window(app);
        }
        Some(TrayMenuAction::StartRecording) => {
            if let Err(error) = handle_start_recording(app) {
                eprintln!("{error}");
            }
            let _ = sync_recording_menu(app);
        }
        Some(TrayMenuAction::StopRecording) => {
            if let Err(error) = handle_stop_recording(app) {
                eprintln!("{error}");
            }
            let _ = sync_recording_menu(app);
        }
        Some(TrayMenuAction::CancelRecording) => {
            if let Err(error) = handle_cancel_recording(app) {
                eprintln!("{error}");
            }
            let _ = sync_recording_menu(app);
        }
        Some(TrayMenuAction::QuitApp) => {
            if let Some(exit_state) = app.try_state::<AppExitState>() {
                exit_state.mark_quitting();
            }
            app.exit(0);
        }
        None => {}
    }
}

fn tray_menu_action(menu_id: &str) -> Option<TrayMenuAction> {
    match menu_id {
        SHOW_APP_MENU_ID => Some(TrayMenuAction::ShowMainWindow),
        START_RECORDING_MENU_ID => Some(TrayMenuAction::StartRecording),
        STOP_RECORDING_MENU_ID => Some(TrayMenuAction::StopRecording),
        CANCEL_RECORDING_MENU_ID => Some(TrayMenuAction::CancelRecording),
        QUIT_APP_MENU_ID => Some(TrayMenuAction::QuitApp),
        _ => None,
    }
}

fn should_restore_for_tray_event(event: &TrayIconEvent) -> bool {
    match event {
        TrayIconEvent::Click {
            button,
            button_state,
            ..
        } => should_restore_on_click(*button, *button_state),
        TrayIconEvent::DoubleClick { button, .. } => should_restore_on_double_click(*button),
        _ => false,
    }
}

fn should_restore_on_click(button: MouseButton, button_state: MouseButtonState) -> bool {
    button == MouseButton::Left && button_state == MouseButtonState::Up
}

fn should_restore_on_double_click(button: MouseButton) -> bool {
    button == MouseButton::Left
}

fn handle_start_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for tray start".to_string());
    };

    recorder_service
        .start(load_preferred_microphone_name(app))
        .map(|_| ())
        .map_err(|error| format!("failed to start recording from tray: {error}"))
}

fn handle_stop_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for tray stop".to_string());
    };

    recorder_service
        .stop()
        .map(|_| ())
        .map_err(|error| format!("failed to stop recording from tray: {error}"))
}

fn handle_cancel_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for tray cancel".to_string());
    };

    recorder_service
        .cancel()
        .map(|_| ())
        .map_err(|error| format!("failed to cancel recording from tray: {error}"))
}

fn apply_recording_menu_state<R: Runtime>(
    app: &AppHandle<R>,
    state: RecordingTrayMenuState,
) -> Result<(), String> {
    if let Some(recording_menu) = app.try_state::<RecordingTrayMenu<R>>() {
        recording_menu
            .apply_state(state)
            .map_err(|error| format!("failed to update tray recording menu: {error}"))?;
    }

    Ok(())
}

fn toggle_recording_action(phase: RecorderPhase) -> Option<ToggleRecordingAction> {
    match phase {
        RecorderPhase::Idle => Some(ToggleRecordingAction::Start),
        RecorderPhase::Recording => Some(ToggleRecordingAction::Stop),
        RecorderPhase::Starting | RecorderPhase::Stopping | RecorderPhase::Cancelling => None,
    }
}

fn preferred_input_available<R: Runtime>(
    app: &AppHandle<R>,
    recorder_service: &RecorderService,
) -> Result<bool, String> {
    let devices = recorder_service
        .list_input_devices()
        .map_err(|error| format!("failed to load recording inputs for tray: {error}"))?;

    Ok(is_preferred_microphone_available(
        load_preferred_microphone_name(app).as_deref(),
        &devices,
    ))
}

fn load_preferred_microphone_name<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    match app.store(SETTINGS_STORE_PATH) {
        Ok(store) => {
            let stored_value = store.get(SELECTED_MICROPHONE_KEY);
            normalize_preferred_microphone_name(
                stored_value.as_ref().and_then(|value| value.as_str()),
            )
        }
        Err(error) => {
            eprintln!("failed to load tray recording settings: {error}");
            None
        }
    }
}

fn normalize_preferred_microphone_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "default")
        .map(ToOwned::to_owned)
}

fn is_preferred_microphone_available(
    preferred_microphone_name: Option<&str>,
    available_devices: &[RecordingInputDevice],
) -> bool {
    match normalize_preferred_microphone_name(preferred_microphone_name) {
        Some(preferred_microphone_name) => available_devices
            .iter()
            .any(|device| device.name == preferred_microphone_name),
        None => available_devices.iter().any(|device| device.is_default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::RecorderSnapshot;

    fn idle_snapshot() -> RecorderSnapshot {
        RecorderSnapshot {
            phase: RecorderPhase::Idle,
            active_session_id: None,
            input_device_name: None,
            elapsed_ms: None,
            remaining_ms: None,
            max_duration_ms: 900_000,
            limit_reached: false,
            last_completed_session_id: None,
        }
    }

    #[test]
    fn maps_supported_tray_menu_actions() {
        assert_eq!(
            tray_menu_action(SHOW_APP_MENU_ID),
            Some(TrayMenuAction::ShowMainWindow)
        );
        assert_eq!(
            tray_menu_action(START_RECORDING_MENU_ID),
            Some(TrayMenuAction::StartRecording)
        );
        assert_eq!(
            tray_menu_action(STOP_RECORDING_MENU_ID),
            Some(TrayMenuAction::StopRecording)
        );
        assert_eq!(
            tray_menu_action(CANCEL_RECORDING_MENU_ID),
            Some(TrayMenuAction::CancelRecording)
        );
        assert_eq!(
            tray_menu_action(QUIT_APP_MENU_ID),
            Some(TrayMenuAction::QuitApp)
        );
        assert_eq!(tray_menu_action("unknown"), None);
    }

    #[test]
    fn only_hides_close_for_main_window_when_not_quitting() {
        assert!(should_hide_on_close(MAIN_WINDOW_LABEL, false));
        assert!(!should_hide_on_close(MAIN_WINDOW_LABEL, true));
        assert!(!should_hide_on_close("settings", false));
    }

    #[test]
    fn restores_for_left_click_release_only() {
        assert!(should_restore_on_click(
            MouseButton::Left,
            MouseButtonState::Up
        ));
        assert!(!should_restore_on_click(
            MouseButton::Left,
            MouseButtonState::Down
        ));
        assert!(!should_restore_on_click(
            MouseButton::Right,
            MouseButtonState::Up
        ));
    }

    #[test]
    fn restores_for_left_button_double_click_only() {
        assert!(should_restore_on_double_click(MouseButton::Left));
        assert!(!should_restore_on_double_click(MouseButton::Right));
    }

    #[test]
    fn recording_menu_enables_start_only_when_idle_and_input_available() {
        assert_eq!(
            RecordingTrayMenuState::from_snapshot(&idle_snapshot(), true),
            RecordingTrayMenuState {
                start_enabled: true,
                stop_enabled: false,
                cancel_enabled: false,
            }
        );
        assert_eq!(
            RecordingTrayMenuState::from_snapshot(&idle_snapshot(), false),
            RecordingTrayMenuState {
                start_enabled: false,
                stop_enabled: false,
                cancel_enabled: false,
            }
        );
    }

    #[test]
    fn recording_menu_enables_stop_and_cancel_while_recording() {
        let mut snapshot = idle_snapshot();
        snapshot.phase = RecorderPhase::Recording;

        assert_eq!(
            RecordingTrayMenuState::from_snapshot(&snapshot, true),
            RecordingTrayMenuState {
                start_enabled: false,
                stop_enabled: true,
                cancel_enabled: true,
            }
        );
    }

    #[test]
    fn recording_menu_disables_actions_during_transitions() {
        for phase in [
            RecorderPhase::Starting,
            RecorderPhase::Stopping,
            RecorderPhase::Cancelling,
        ] {
            let mut snapshot = idle_snapshot();
            snapshot.phase = phase;

            assert_eq!(
                RecordingTrayMenuState::from_snapshot(&snapshot, true),
                RecordingTrayMenuState {
                    start_enabled: false,
                    stop_enabled: false,
                    cancel_enabled: false,
                }
            );
        }
    }

    #[test]
    fn normalizes_preferred_microphone_name() {
        assert_eq!(normalize_preferred_microphone_name(None), None);
        assert_eq!(normalize_preferred_microphone_name(Some("")), None);
        assert_eq!(normalize_preferred_microphone_name(Some("  ")), None);
        assert_eq!(normalize_preferred_microphone_name(Some("default")), None);
        assert_eq!(
            normalize_preferred_microphone_name(Some("  Podcast Mic  ")),
            Some("Podcast Mic".to_string())
        );
    }

    #[test]
    fn detects_preferred_microphone_availability() {
        let devices = vec![
            RecordingInputDevice {
                name: "Built-in Mic".to_string(),
                is_default: true,
            },
            RecordingInputDevice {
                name: "Podcast Mic".to_string(),
                is_default: false,
            },
        ];

        assert!(is_preferred_microphone_available(None, &devices));
        assert!(is_preferred_microphone_available(Some("default"), &devices));
        assert!(is_preferred_microphone_available(
            Some("Podcast Mic"),
            &devices
        ));
        assert!(!is_preferred_microphone_available(
            Some("Unavailable Mic"),
            &devices
        ));
    }

    #[test]
    fn determines_toggle_recording_action_from_phase() {
        assert_eq!(
            toggle_recording_action(RecorderPhase::Idle),
            Some(ToggleRecordingAction::Start)
        );
        assert_eq!(
            toggle_recording_action(RecorderPhase::Recording),
            Some(ToggleRecordingAction::Stop)
        );

        for phase in [
            RecorderPhase::Starting,
            RecorderPhase::Stopping,
            RecorderPhase::Cancelling,
        ] {
            assert_eq!(toggle_recording_action(phase), None);
        }
    }
}
