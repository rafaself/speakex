use std::sync::Arc;

use crate::app_settings::{load_selected_microphone_name, load_transcription_settings};
use crate::history_database::HistoryDatabase;
use crate::history_repository::HistoryRepository;
use crate::manual_flow::{ManualTranscriptionFlow, ManualTranscriptionSettings};
use crate::notifications;
use crate::recorder::{RecorderPhase, RecorderService, StoppedRecording};
use crate::secret_store::SecretStoreService;
use crate::transcription::{GeminiProvider, TranscriptionService};
use crate::tray::{self, MAIN_WINDOW_LABEL};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShortcutToggleAction {
    Start,
    Stop,
}

pub fn toggle_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for shortcut toggle".to_string());
    };

    let action = shortcut_toggle_action(
        recorder_service
            .snapshot()
            .map_err(|error| format!("failed to read recording status for shortcut: {error}"))?
            .phase,
    )
    .ok_or_else(|| "recording cannot be toggled while the recorder is transitioning".to_string())?;

    let result = match action {
        ShortcutToggleAction::Start => start_recording(app),
        ShortcutToggleAction::Stop => stop_recording(app),
    };
    let _ = tray::sync_recording_menu(app);

    result
}

fn start_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for shortcut start".to_string());
    };

    recorder_service
        .start(load_selected_microphone_name(app))
        .map(|_| ())
        .map_err(|error| format!("failed to start recording from shortcut: {error}"))
}

fn stop_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(recorder_service) = app.try_state::<RecorderService>() else {
        return Err("recorder service is unavailable for shortcut stop".to_string());
    };

    let stopped_recording = recorder_service
        .stop()
        .map_err(|error| format!("failed to stop recording from shortcut: {error}"))?;

    if !should_transcribe_and_paste_after_shortcut_stop(app)? {
        return Ok(());
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            transcribe_and_paste_shortcut_recording(app_handle.clone(), stopped_recording).await
        {
            notifications::notify_shortcut_transcription_failed(&app_handle);
            eprintln!("failed to finish shortcut transcription flow: {error}");
        }
    });

    Ok(())
}

fn should_transcribe_and_paste_after_shortcut_stop<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<bool, String> {
    let settings = load_transcription_settings(app)?;

    if !settings.paste_after_shortcut_recording {
        return Ok(false);
    }

    is_main_window_inactive(app)
}

fn is_main_window_inactive<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return Ok(true);
    };

    let is_visible = window
        .is_visible()
        .map_err(|error| format!("failed to read main window visibility: {error}"))?;

    if !is_visible {
        return Ok(true);
    }

    let is_focused = window
        .is_focused()
        .map_err(|error| format!("failed to read main window focus: {error}"))?;

    Ok(!is_focused)
}

async fn transcribe_and_paste_shortcut_recording<R: Runtime>(
    app: AppHandle<R>,
    stopped_recording: StoppedRecording,
) -> Result<(), String> {
    let history_database = app
        .try_state::<HistoryDatabase>()
        .ok_or_else(|| "history database is unavailable for shortcut transcription".to_string())?;
    let secret_store_service = app.try_state::<SecretStoreService>().ok_or_else(|| {
        "secret store service is unavailable for shortcut transcription".to_string()
    })?;
    let settings = load_transcription_settings(&app)?;
    let flow = ManualTranscriptionFlow::new(
        TranscriptionService::new(Arc::new(GeminiProvider::new(
            secret_store_service.inner().clone(),
        ))),
        HistoryRepository::new(history_database.inner().clone()),
        app.clone(),
    );
    let result = flow
        .run(
            stopped_recording.audio_input,
            ManualTranscriptionSettings {
                default_language: settings.default_language,
                auto_copy: settings.auto_copy,
                save_audio_files: settings.save_audio_files,
                save_transcription_history: settings.save_transcription_history,
                paste_after_transcription: true,
            },
        )
        .await?;

    if !result.pasted_to_active_input {
        if let Some(error) = result.paste_error {
            return Err(format!("automatic paste failed: {error}"));
        }

        if let Some(error) = result.clipboard_error {
            return Err(format!(
                "automatic paste could not write the transcript to the clipboard: {error}"
            ));
        }

        return Err("automatic paste did not complete after shortcut transcription".to_string());
    }

    if let Some(history_error) = result.history_error {
        eprintln!("shortcut transcription history warning: {history_error}");
    }

    if let Some(audio_delete_error) = result.audio_delete_error {
        eprintln!("shortcut transcription audio cleanup warning: {audio_delete_error}");
    }

    Ok(())
}

fn shortcut_toggle_action(phase: RecorderPhase) -> Option<ShortcutToggleAction> {
    match phase {
        RecorderPhase::Idle => Some(ShortcutToggleAction::Start),
        RecorderPhase::Recording => Some(ShortcutToggleAction::Stop),
        RecorderPhase::Starting | RecorderPhase::Stopping | RecorderPhase::Cancelling => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{shortcut_toggle_action, ShortcutToggleAction};
    use crate::recorder::RecorderPhase;

    #[test]
    fn shortcut_toggle_action_matches_recording_phase() {
        assert_eq!(
            shortcut_toggle_action(RecorderPhase::Idle),
            Some(ShortcutToggleAction::Start)
        );
        assert_eq!(
            shortcut_toggle_action(RecorderPhase::Recording),
            Some(ShortcutToggleAction::Stop)
        );

        for phase in [
            RecorderPhase::Starting,
            RecorderPhase::Stopping,
            RecorderPhase::Cancelling,
        ] {
            assert_eq!(shortcut_toggle_action(phase), None);
        }
    }
}
