use crate::history_repository::{HistoryRepository, NewHistoryTranscription};
use crate::transcription::{AudioInput, Transcript, TranscriptionOptions, TranscriptionService};
use serde::Serialize;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualTranscriptionSettings {
    pub default_language: Option<String>,
    pub auto_copy: bool,
    pub save_audio_files: bool,
    pub save_transcription_history: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunCompletedRecordingTranscriptionResult {
    pub transcript: Transcript,
    pub history_id: Option<String>,
    pub history_saved: bool,
    pub history_error: Option<String>,
    pub copied_to_clipboard: bool,
    pub clipboard_error: Option<String>,
    pub audio_deleted: bool,
    pub audio_delete_error: Option<String>,
    pub retained_audio_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct ManualTranscriptionFlow {
    transcription_service: TranscriptionService,
    history_repository: Arc<dyn HistorySink>,
    clipboard_writer: Arc<dyn ClipboardSink>,
    file_system: Arc<dyn FileSystem>,
}

impl ManualTranscriptionFlow {
    pub fn new(
        transcription_service: TranscriptionService,
        history_repository: HistoryRepository,
        app: AppHandle,
    ) -> Self {
        Self::with_dependencies(
            transcription_service,
            Arc::new(history_repository),
            Arc::new(AppClipboardSink::new(app)),
            Arc::new(StdFileSystem),
        )
    }

    fn with_dependencies(
        transcription_service: TranscriptionService,
        history_repository: Arc<dyn HistorySink>,
        clipboard_writer: Arc<dyn ClipboardSink>,
        file_system: Arc<dyn FileSystem>,
    ) -> Self {
        Self {
            transcription_service,
            history_repository,
            clipboard_writer,
            file_system,
        }
    }

    pub async fn run(
        &self,
        audio_input: AudioInput,
        settings: ManualTranscriptionSettings,
    ) -> Result<RunCompletedRecordingTranscriptionResult, String> {
        if !self.file_system.is_file(&audio_input.path) {
            return Err(format!(
                "The completed recording is no longer available at {}.",
                audio_input.path.display()
            ));
        }

        let transcript = self
            .transcription_service
            .transcribe(
                audio_input.clone(),
                TranscriptionOptions {
                    language: settings.default_language,
                    prompt: None,
                    model: None,
                },
            )
            .await
            .map_err(|error| format!("Gemini transcription failed: {error}"))?;

        let clipboard_error = if settings.auto_copy {
            self.clipboard_writer.write_text(&transcript.text).err()
        } else {
            None
        };
        let copied_to_clipboard = settings.auto_copy && clipboard_error.is_none();

        let audio_cleanup = if settings.save_audio_files {
            AudioCleanupOutcome::retained(audio_input.path.clone())
        } else {
            AudioCleanupOutcome::delete(self.file_system.as_ref(), &audio_input.path)
        };

        let history_outcome = if settings.save_transcription_history {
            self.save_history_entry(
                &audio_input,
                &transcript,
                copied_to_clipboard,
                clipboard_error.as_deref(),
                &audio_cleanup,
            )
        } else {
            HistorySaveOutcome::skipped()
        };

        Ok(RunCompletedRecordingTranscriptionResult {
            transcript,
            history_id: history_outcome.id,
            history_saved: history_outcome.saved,
            history_error: history_outcome.error,
            copied_to_clipboard,
            clipboard_error,
            audio_deleted: audio_cleanup.deleted,
            audio_delete_error: audio_cleanup.error,
            retained_audio_path: audio_cleanup.retained_audio_path,
        })
    }

    fn save_history_entry(
        &self,
        _audio_input: &AudioInput,
        transcript: &Transcript,
        copied_to_clipboard: bool,
        clipboard_error: Option<&str>,
        audio_cleanup: &AudioCleanupOutcome,
    ) -> HistorySaveOutcome {
        let id = match generate_history_entry_id(&transcript.provider) {
            Ok(id) => id,
            Err(error) => return HistorySaveOutcome::failed(error),
        };
        let duration_ms = match normalize_duration_ms(transcript.duration_ms) {
            Ok(duration_ms) => duration_ms,
            Err(error) => return HistorySaveOutcome::failed(error),
        };
        let error = join_outcome_errors(clipboard_error, audio_cleanup.error.as_deref());
        let entry = NewHistoryTranscription {
            id: id.clone(),
            text: transcript.text.clone(),
            provider: transcript.provider.clone(),
            model: transcript.model.clone(),
            language: transcript.language.clone(),
            duration_ms,
            audio_path: audio_cleanup
                .retained_audio_path
                .as_ref()
                .map(|path| path.display().to_string()),
            audio_deleted: audio_cleanup.deleted,
            copied_to_clipboard,
            error,
        };

        match self.history_repository.save_transcription(&entry) {
            Ok(()) => HistorySaveOutcome::saved(id),
            Err(error) => {
                HistorySaveOutcome::failed(format!("failed to save transcription history: {error}"))
            }
        }
    }
}

trait HistorySink: Send + Sync {
    fn save_transcription(&self, entry: &NewHistoryTranscription) -> Result<(), String>;
}

impl HistorySink for HistoryRepository {
    fn save_transcription(&self, entry: &NewHistoryTranscription) -> Result<(), String> {
        HistoryRepository::save_transcription(self, entry)
    }
}

trait ClipboardSink: Send + Sync {
    fn write_text(&self, text: &str) -> Result<(), String>;
}

struct AppClipboardSink {
    app: AppHandle,
}

impl AppClipboardSink {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ClipboardSink for AppClipboardSink {
    fn write_text(&self, text: &str) -> Result<(), String> {
        self.app
            .clipboard()
            .write_text(text.to_string())
            .map_err(|error| format!("failed to copy transcript to the system clipboard: {error}"))
    }
}

trait FileSystem: Send + Sync {
    fn is_file(&self, path: &Path) -> bool;
    fn remove_file(&self, path: &Path) -> Result<(), String>;
}

struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        local_audio_file_exists(path)
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "failed to delete local audio file {}: {error}",
                path.display()
            )),
        }
    }
}

pub fn local_audio_file_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

#[derive(Clone, Debug)]
struct AudioCleanupOutcome {
    deleted: bool,
    retained_audio_path: Option<PathBuf>,
    error: Option<String>,
}

impl AudioCleanupOutcome {
    fn retained(path: PathBuf) -> Self {
        Self {
            deleted: false,
            retained_audio_path: Some(path),
            error: None,
        }
    }

    fn delete(file_system: &dyn FileSystem, path: &Path) -> Self {
        match file_system.remove_file(path) {
            Ok(()) => Self {
                deleted: true,
                retained_audio_path: None,
                error: None,
            },
            Err(error) => Self {
                deleted: false,
                retained_audio_path: Some(path.to_path_buf()),
                error: Some(error),
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
struct HistorySaveOutcome {
    id: Option<String>,
    saved: bool,
    error: Option<String>,
}

impl HistorySaveOutcome {
    fn skipped() -> Self {
        Self::default()
    }

    fn saved(id: String) -> Self {
        Self {
            id: Some(id),
            saved: true,
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            id: None,
            saved: false,
            error: Some(error),
        }
    }
}

fn normalize_duration_ms(duration_ms: Option<u64>) -> Result<Option<i64>, String> {
    duration_ms
        .map(|value| {
            i64::try_from(value).map_err(|_| {
                format!("transcription duration {value}ms exceeds supported history range")
            })
        })
        .transpose()
}

fn generate_history_entry_id(provider: &str) -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to generate transcription history identifier: {error}"))?;

    Ok(format!(
        "{}-{}-{}",
        provider.trim().to_lowercase(),
        timestamp.as_secs(),
        timestamp.subsec_nanos()
    ))
}

fn join_outcome_errors(first: Option<&str>, second: Option<&str>) -> Option<String> {
    let errors = [first, second]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (!errors.is_empty()).then(|| errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::{
        ClipboardSink, FileSystem, HistorySink, ManualTranscriptionFlow,
        ManualTranscriptionSettings,
    };
    use crate::history_repository::NewHistoryTranscription;
    use crate::transcription::{
        AudioInput, Transcript, TranscriptionOptions, TranscriptionProvider,
        TranscriptionProviderError, TranscriptionService,
    };
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    #[test]
    fn run_saves_history_copies_clipboard_and_deletes_audio_by_default() {
        let clipboard = Arc::new(FakeClipboard::default());
        let history = Arc::new(FakeHistoryRepository::default());
        let file_system = Arc::new(FakeFileSystem::default());
        let flow = manual_flow_for_tests(
            Arc::new(FakeTranscriptionProvider::succeed_with("Transcript text")),
            history.clone(),
            clipboard.clone(),
            file_system.clone(),
        );
        let audio_input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210));

        let result = tauri::async_runtime::block_on(flow.run(
            audio_input.clone(),
            ManualTranscriptionSettings {
                default_language: Some("en-US".to_string()),
                auto_copy: true,
                save_audio_files: false,
                save_transcription_history: true,
            },
        ))
        .expect("manual flow should succeed");

        assert_eq!(result.transcript.text, "Transcript text");
        assert_eq!(result.transcript.language.as_deref(), Some("en-US"));
        assert!(result.history_saved);
        assert!(result.history_id.is_some());
        assert!(result.copied_to_clipboard);
        assert_eq!(result.clipboard_error, None);
        assert!(result.audio_deleted);
        assert_eq!(result.audio_delete_error, None);
        assert_eq!(result.retained_audio_path, None);
        assert_eq!(
            clipboard
                .writes
                .lock()
                .expect("clipboard writes lock should succeed")
                .as_slice(),
            &[String::from("Transcript text")]
        );
        assert_eq!(
            file_system
                .deleted_paths
                .lock()
                .expect("deleted paths lock should succeed")
                .as_slice(),
            &[PathBuf::from("recording.wav")]
        );

        let saved_entries = history
            .saved_entries
            .lock()
            .expect("saved entries lock should succeed");
        assert_eq!(saved_entries.len(), 1);
        assert_eq!(saved_entries[0].text, "Transcript text");
        assert_eq!(saved_entries[0].language.as_deref(), Some("en-US"));
        assert_eq!(saved_entries[0].audio_path, None);
        assert!(saved_entries[0].audio_deleted);
        assert!(saved_entries[0].copied_to_clipboard);
        assert_eq!(saved_entries[0].error, None);
    }

    #[test]
    fn run_retains_audio_and_skips_optional_side_effects_when_disabled() {
        let clipboard = Arc::new(FakeClipboard::default());
        let history = Arc::new(FakeHistoryRepository::default());
        let file_system = Arc::new(FakeFileSystem::default());
        let flow = manual_flow_for_tests(
            Arc::new(FakeTranscriptionProvider::succeed_with("Transcript text")),
            history.clone(),
            clipboard.clone(),
            file_system.clone(),
        );
        let audio_input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210));

        let result = tauri::async_runtime::block_on(flow.run(
            audio_input.clone(),
            ManualTranscriptionSettings {
                default_language: None,
                auto_copy: false,
                save_audio_files: true,
                save_transcription_history: false,
            },
        ))
        .expect("manual flow should succeed");

        assert!(!result.history_saved);
        assert_eq!(result.history_id, None);
        assert_eq!(result.history_error, None);
        assert!(!result.copied_to_clipboard);
        assert_eq!(result.clipboard_error, None);
        assert!(!result.audio_deleted);
        assert_eq!(result.audio_delete_error, None);
        assert_eq!(
            result.retained_audio_path,
            Some(PathBuf::from("recording.wav"))
        );
        assert!(clipboard
            .writes
            .lock()
            .expect("clipboard writes lock should succeed")
            .is_empty());
        assert!(history
            .saved_entries
            .lock()
            .expect("saved entries lock should succeed")
            .is_empty());
        assert!(file_system
            .deleted_paths
            .lock()
            .expect("deleted paths lock should succeed")
            .is_empty());
    }

    #[test]
    fn run_surfaces_non_fatal_clipboard_and_audio_cleanup_errors() {
        let clipboard = Arc::new(FakeClipboard {
            error: Mutex::new(Some("clipboard unavailable".to_string())),
            ..Default::default()
        });
        let history = Arc::new(FakeHistoryRepository::default());
        let file_system = Arc::new(FakeFileSystem {
            delete_error: Mutex::new(Some("permission denied".to_string())),
            ..Default::default()
        });
        let flow = manual_flow_for_tests(
            Arc::new(FakeTranscriptionProvider::succeed_with("Transcript text")),
            history.clone(),
            clipboard,
            file_system,
        );
        let audio_input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210));

        let result = tauri::async_runtime::block_on(flow.run(
            audio_input.clone(),
            ManualTranscriptionSettings {
                default_language: None,
                auto_copy: true,
                save_audio_files: false,
                save_transcription_history: true,
            },
        ))
        .expect("manual flow should keep transcript results even when side effects fail");

        assert!(!result.copied_to_clipboard);
        assert_eq!(
            result.clipboard_error.as_deref(),
            Some("clipboard unavailable")
        );
        assert!(!result.audio_deleted);
        assert_eq!(
            result.audio_delete_error.as_deref(),
            Some("permission denied")
        );
        assert_eq!(
            result.retained_audio_path,
            Some(PathBuf::from("recording.wav"))
        );
        assert!(result.history_saved);

        let saved_entries = history
            .saved_entries
            .lock()
            .expect("saved entries lock should succeed");
        assert_eq!(
            saved_entries[0].audio_path.as_deref(),
            Some("recording.wav")
        );
        assert!(!saved_entries[0].audio_deleted);
        assert!(!saved_entries[0].copied_to_clipboard);
        assert_eq!(
            saved_entries[0].error.as_deref(),
            Some("clipboard unavailable; permission denied")
        );
    }

    #[test]
    fn run_reports_history_save_failures_without_failing_transcription() {
        let history = Arc::new(FakeHistoryRepository {
            save_error: Mutex::new(Some("database busy".to_string())),
            ..Default::default()
        });
        let flow = manual_flow_for_tests(
            Arc::new(FakeTranscriptionProvider::succeed_with("Transcript text")),
            history.clone(),
            Arc::new(FakeClipboard::default()),
            Arc::new(FakeFileSystem::default()),
        );

        let result = tauri::async_runtime::block_on(flow.run(
            AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210)),
            ManualTranscriptionSettings {
                default_language: None,
                auto_copy: false,
                save_audio_files: false,
                save_transcription_history: true,
            },
        ))
        .expect("manual flow should still return the transcript");

        assert!(!result.history_saved);
        assert_eq!(result.history_id, None);
        assert_eq!(
            result.history_error.as_deref(),
            Some("failed to save transcription history: database busy")
        );
        assert!(history
            .saved_entries
            .lock()
            .expect("saved entries lock should succeed")
            .is_empty());
    }

    #[test]
    fn run_returns_error_when_local_audio_file_is_missing() {
        let clipboard = Arc::new(FakeClipboard::default());
        let history = Arc::new(FakeHistoryRepository::default());
        let file_system = Arc::new(FakeFileSystem {
            file_exists: Mutex::new(false),
            ..Default::default()
        });
        let flow = manual_flow_for_tests(
            Arc::new(FakeTranscriptionProvider::succeed_with("Transcript text")),
            history.clone(),
            clipboard.clone(),
            file_system.clone(),
        );

        let error = tauri::async_runtime::block_on(flow.run(
            AudioInput::new(PathBuf::from("missing.wav"), "audio/wav", Some(3210)),
            ManualTranscriptionSettings {
                default_language: None,
                auto_copy: true,
                save_audio_files: false,
                save_transcription_history: true,
            },
        ))
        .expect_err("manual flow should fail when the local audio file is missing");

        assert_eq!(
            error,
            "The completed recording is no longer available at missing.wav."
        );
        assert!(clipboard
            .writes
            .lock()
            .expect("clipboard writes lock should succeed")
            .is_empty());
        assert!(history
            .saved_entries
            .lock()
            .expect("saved entries lock should succeed")
            .is_empty());
        assert!(file_system
            .deleted_paths
            .lock()
            .expect("deleted paths lock should succeed")
            .is_empty());
    }

    fn manual_flow_for_tests(
        provider: Arc<dyn TranscriptionProvider>,
        history_repository: Arc<dyn HistorySink>,
        clipboard_writer: Arc<dyn ClipboardSink>,
        file_system: Arc<dyn FileSystem>,
    ) -> ManualTranscriptionFlow {
        ManualTranscriptionFlow::with_dependencies(
            TranscriptionService::new(provider),
            history_repository,
            clipboard_writer,
            file_system,
        )
    }

    struct FakeTranscriptionProvider {
        transcript_text: String,
    }

    impl FakeTranscriptionProvider {
        fn succeed_with(text: &str) -> Self {
            Self {
                transcript_text: text.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl TranscriptionProvider for FakeTranscriptionProvider {
        async fn transcribe(
            &self,
            input: AudioInput,
            options: TranscriptionOptions,
        ) -> Result<Transcript, TranscriptionProviderError> {
            Ok(Transcript {
                text: self.transcript_text.clone(),
                provider: "gemini".to_string(),
                model: Some("gemini-2.0-flash".to_string()),
                language: options.language,
                duration_ms: input.duration_ms,
            })
        }

        fn name(&self) -> &'static str {
            "gemini"
        }

        fn capabilities(&self) -> crate::transcription::ProviderCapabilities {
            crate::transcription::ProviderCapabilities::new(true, false, false)
        }
    }

    #[derive(Default)]
    struct FakeHistoryRepository {
        saved_entries: Mutex<Vec<NewHistoryTranscription>>,
        save_error: Mutex<Option<String>>,
    }

    impl HistorySink for FakeHistoryRepository {
        fn save_transcription(&self, entry: &NewHistoryTranscription) -> Result<(), String> {
            if let Some(error) = self
                .save_error
                .lock()
                .expect("save_error lock should succeed")
                .take()
            {
                return Err(error);
            }

            self.saved_entries
                .lock()
                .expect("saved_entries lock should succeed")
                .push(entry.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeClipboard {
        writes: Mutex<Vec<String>>,
        error: Mutex<Option<String>>,
    }

    impl ClipboardSink for FakeClipboard {
        fn write_text(&self, text: &str) -> Result<(), String> {
            if let Some(error) = self.error.lock().expect("error lock should succeed").take() {
                return Err(error);
            }

            self.writes
                .lock()
                .expect("writes lock should succeed")
                .push(text.to_string());
            Ok(())
        }
    }

    struct FakeFileSystem {
        file_exists: Mutex<bool>,
        deleted_paths: Mutex<Vec<PathBuf>>,
        delete_error: Mutex<Option<String>>,
    }

    impl Default for FakeFileSystem {
        fn default() -> Self {
            Self {
                file_exists: Mutex::new(true),
                deleted_paths: Mutex::new(Vec::new()),
                delete_error: Mutex::new(None),
            }
        }
    }

    impl FileSystem for FakeFileSystem {
        fn is_file(&self, _path: &Path) -> bool {
            *self
                .file_exists
                .lock()
                .expect("file_exists lock should succeed")
        }

        fn remove_file(&self, path: &Path) -> Result<(), String> {
            if let Some(error) = self
                .delete_error
                .lock()
                .expect("delete_error lock should succeed")
                .take()
            {
                return Err(error);
            }

            self.deleted_paths
                .lock()
                .expect("deleted_paths lock should succeed")
                .push(path.to_path_buf());
            Ok(())
        }
    }
}
