mod app_settings;
mod commands;
pub mod history_database;
pub mod history_repository;
pub mod manual_flow;
mod notifications;
pub mod recorder;
pub mod secret_store;
pub mod shortcut;
mod shortcut_flow;
pub mod transcription;
pub mod tray;

use recorder::RecorderService;
use secret_store::SecretStoreService;
use shortcut::ShortcutService;
use std::io;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if tray::should_hide_on_close(window.label(), tray::is_quitting(window)) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let history_database =
                history_database::initialize(app.handle()).map_err(io::Error::other)?;

            app.manage(history_database);
            let recordings_dir = app
                .path()
                .app_cache_dir()
                .map_err(io::Error::other)?
                .join("recordings");

            app.manage(RecorderService::new(recordings_dir));
            app.manage(SecretStoreService::new());
            app.manage(tray::AppExitState::default());
            app.manage(ShortcutService::default());
            tray::initialize(app.handle()).map_err(io::Error::other)?;
            shortcut::initialize(app.handle()).map_err(io::Error::other)?;

            Ok(())
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::ping,
            commands::history::get_history,
            commands::history::get_transcription,
            commands::history::delete_transcription,
            commands::history::clear_history,
            commands::history::get_error_logs,
            commands::history::clear_error_logs,
            commands::history::create_error_log,
            commands::recording::list_recording_input_devices,
            commands::recording::start_recording,
            commands::recording::get_recording_status,
            commands::recording::stop_recording,
            commands::recording::cancel_recording,
            commands::settings::get_recording_shortcut_status,
            commands::settings::apply_recording_shortcut,
            commands::settings::save_gemini_api_key,
            commands::settings::has_gemini_api_key,
            commands::settings::clear_gemini_api_key,
            commands::transcription::run_gemini_transcription,
            commands::transcription::run_completed_recording_transcription,
            commands::transcription::has_completed_recording_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
