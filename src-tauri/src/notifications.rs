use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::tray::MAIN_WINDOW_LABEL;

const COMPLETION_TITLE: &str = "Transcription complete";
const COMPLETION_BODY: &str = "Your recording has been transcribed.";
const FAILURE_TITLE: &str = "Transcription failed";
const FAILURE_FALLBACK_BODY: &str = "SpeakEx could not transcribe your recording.";

pub fn notify_manual_transcription_completed<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = show_if_main_window_hidden(app, COMPLETION_TITLE, COMPLETION_BODY) {
        eprintln!("failed to send transcription completion notification: {error}");
    }
}

pub fn notify_manual_transcription_failed<R: Runtime>(app: &AppHandle<R>, error: &str) {
    if let Err(notification_error) =
        show_if_main_window_hidden(app, FAILURE_TITLE, notification_failure_body(error))
    {
        eprintln!("failed to send transcription failure notification: {notification_error}");
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

fn notification_failure_body(error: &str) -> String {
    let trimmed = error.trim();

    if trimmed.is_empty() {
        FAILURE_FALLBACK_BODY.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        notification_failure_body, should_show_for_window_visibility, FAILURE_FALLBACK_BODY,
    };

    #[test]
    fn only_shows_notifications_when_main_window_is_hidden() {
        assert!(should_show_for_window_visibility(false));
        assert!(!should_show_for_window_visibility(true));
    }

    #[test]
    fn failure_notifications_fallback_when_error_is_blank() {
        assert_eq!(
            notification_failure_body("   "),
            FAILURE_FALLBACK_BODY.to_string()
        );
        assert_eq!(
            notification_failure_body(" Gemini transcription failed "),
            "Gemini transcription failed".to_string()
        );
    }
}
