use crate::recorder::{
    ActiveRecordingSession, CancelledRecording, RecorderService, RecorderSnapshot,
    RecordingInputDevice, StoppedRecording,
};
use crate::tray;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_recording_input_devices(
    recorder_service: State<'_, RecorderService>,
) -> Result<Vec<RecordingInputDevice>, String> {
    recorder_service
        .list_input_devices()
        .map_err(|error| format!("failed to list recording input devices: {error}"))
}

#[tauri::command]
pub fn start_recording(
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
pub fn get_recording_status(
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
pub fn stop_recording(
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
pub fn cancel_recording(
    app: AppHandle,
    recorder_service: State<'_, RecorderService>,
) -> Result<CancelledRecording, String> {
    let result = recorder_service
        .cancel()
        .map_err(|error| format!("failed to cancel recording: {error}"));
    let _ = tray::sync_recording_menu(&app);

    result
}