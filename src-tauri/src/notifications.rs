use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::tray::MAIN_WINDOW_LABEL;

const COMPLETION_TITLE: &str = "Transcription complete";
const COMPLETION_BODY: &str = "Your recording has been transcribed.";
const FAILURE_TITLE: &str = "Transcription failed";
const FAILURE_BODY: &str = "SpeakEx could not transcribe your recording. Open the app for details.";

pub fn notify_manual_transcription_completed<R: Runtime>(app: &AppHandle<R>) {
    if show_if_main_window_hidden(app, COMPLETION_TITLE, COMPLETION_BODY).is_err() {
        eprintln!("failed to send transcription completion notification");
    }
}

pub fn notify_manual_transcription_failed<R: Runtime>(app: &AppHandle<R>) {
    if show_if_main_window_hidden(app, FAILURE_TITLE, notification_failure_body()).is_err() {
        eprintln!("failed to send transcription failure notification");
    }
}

fn show_if_main_window_hidden<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: impl Into<String>,
) -> Result<(), String> {
    if !should_show_for_hidden_main_window(app)? {
        return Ok(());
    }

    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| format!("notification delivery failed: {error}"))
}

fn should_show_for_hidden_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return Ok(false);
    };

    window
        .is_visible()
        .map(should_show_for_window_visibility)
        .map_err(|error| format!("failed to read main window visibility: {error}"))
}

fn should_show_for_window_visibility(is_visible: bool) -> bool {
    !is_visible
}

fn notification_failure_body() -> String {
    FAILURE_BODY.to_string()
}

#[cfg(test)]
mod tests {
    use super::{notification_failure_body, should_show_for_window_visibility, FAILURE_BODY};

    #[test]
    fn only_shows_notifications_when_main_window_is_hidden() {
        assert!(should_show_for_window_visibility(false));
        assert!(!should_show_for_window_visibility(true));
    }

    #[test]
    fn failure_notifications_use_generic_privacy_safe_body() {
        assert_eq!(notification_failure_body(), FAILURE_BODY.to_string());
    }
}
