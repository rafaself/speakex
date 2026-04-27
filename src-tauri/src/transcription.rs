use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioInput {
    pub path: PathBuf,
    pub mime_type: String,
    pub duration_ms: Option<u64>,
}

impl AudioInput {
    pub fn new(path: PathBuf, mime_type: impl Into<String>, duration_ms: Option<u64>) -> Self {
        Self {
            path,
            mime_type: mime_type.into(),
            duration_ms,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TranscriptionOptions {
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub supports_language_hint: bool,
    pub supports_local_execution: bool,
    pub supports_timestamps: bool,
}

impl ProviderCapabilities {
    pub const fn new(
        supports_language_hint: bool,
        supports_local_execution: bool,
        supports_timestamps: bool,
    ) -> Self {
        Self {
            supports_language_hint,
            supports_local_execution,
            supports_timestamps,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProviderError {
    pub message: String,
}

impl TranscriptionProviderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TranscriptionProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TranscriptionProviderError {}

impl From<String> for TranscriptionProviderError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for TranscriptionProviderError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

#[async_trait::async_trait]
pub trait TranscriptionProvider: Send + Sync {
    async fn transcribe(
        &self,
        input: AudioInput,
        options: TranscriptionOptions,
    ) -> Result<Transcript, TranscriptionProviderError>;

    fn name(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;
}

pub const MOCK_PROVIDER_NAME: &str = "mock";
pub const MOCK_PROVIDER_MODEL: &str = "mock-local-v1";

#[derive(Clone, Debug, Default)]
pub struct MockTranscriptionProvider;

impl MockTranscriptionProvider {
    pub fn new() -> Self {
        Self
    }

    fn build_fake_text(input: &AudioInput) -> String {
        let source_name = input
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("audio input");

        format!("Mock transcript for {source_name}")
    }
}

#[async_trait::async_trait]
impl TranscriptionProvider for MockTranscriptionProvider {
    async fn transcribe(
        &self,
        input: AudioInput,
        options: TranscriptionOptions,
    ) -> Result<Transcript, TranscriptionProviderError> {
        Ok(Transcript {
            text: Self::build_fake_text(&input),
            provider: self.name().to_string(),
            model: options
                .model
                .or_else(|| Some(MOCK_PROVIDER_MODEL.to_string())),
            language: options.language,
            duration_ms: input.duration_ms,
        })
    }

    fn name(&self) -> &'static str {
        MOCK_PROVIDER_NAME
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::new(true, true, false)
    }
}

#[derive(Clone)]
pub struct TranscriptionService {
    provider: Arc<dyn TranscriptionProvider>,
}

impl TranscriptionService {
    pub fn new(provider: Arc<dyn TranscriptionProvider>) -> Self {
        Self { provider }
    }

    pub async fn transcribe(
        &self,
        input: AudioInput,
        options: TranscriptionOptions,
    ) -> Result<Transcript, TranscriptionProviderError> {
        self.provider.transcribe(input, options).await
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub fn provider_capabilities(&self) -> ProviderCapabilities {
        self.provider.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioInput, MockTranscriptionProvider, ProviderCapabilities, TranscriptionOptions,
        TranscriptionProvider, TranscriptionProviderError, TranscriptionService,
        MOCK_PROVIDER_MODEL, MOCK_PROVIDER_NAME,
    };
    use std::{path::PathBuf, sync::Arc};

    #[test]
    fn audio_input_constructor_sets_fields() {
        let input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(1234));

        assert_eq!(input.path, PathBuf::from("recording.wav"));
        assert_eq!(input.mime_type, "audio/wav");
        assert_eq!(input.duration_ms, Some(1234));
    }

    #[test]
    fn transcription_options_default_to_empty_values() {
        assert_eq!(TranscriptionOptions::default().language, None);
        assert_eq!(TranscriptionOptions::default().prompt, None);
        assert_eq!(TranscriptionOptions::default().model, None);
    }

    #[test]
    fn provider_capabilities_constructor_sets_flags() {
        let capabilities = ProviderCapabilities::new(true, false, true);

        assert!(capabilities.supports_language_hint);
        assert!(!capabilities.supports_local_execution);
        assert!(capabilities.supports_timestamps);
    }

    #[test]
    fn provider_error_formats_message() {
        let error = TranscriptionProviderError::new("provider failed");

        assert_eq!(error.to_string(), "provider failed");
    }

    #[test]
    fn mock_provider_exposes_expected_metadata() {
        let provider = MockTranscriptionProvider::new();

        assert_eq!(provider.name(), MOCK_PROVIDER_NAME);
        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities::new(true, true, false)
        );
    }

    #[test]
    fn mock_provider_returns_fake_transcript() {
        let provider = MockTranscriptionProvider::new();
        let input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(1234));
        let options = TranscriptionOptions {
            language: Some("en-US".to_string()),
            prompt: Some("Summarize clearly".to_string()),
            model: None,
        };

        let transcript = tauri::async_runtime::block_on(provider.transcribe(input, options))
            .expect("mock provider should succeed");

        assert_eq!(transcript.text, "Mock transcript for recording.wav");
        assert_eq!(transcript.provider, MOCK_PROVIDER_NAME);
        assert_eq!(transcript.model.as_deref(), Some(MOCK_PROVIDER_MODEL));
        assert_eq!(transcript.language.as_deref(), Some("en-US"));
        assert_eq!(transcript.duration_ms, Some(1234));
    }

    #[test]
    fn transcription_service_delegates_to_provider() {
        let service = TranscriptionService::new(Arc::new(MockTranscriptionProvider::new()));
        let input = AudioInput::new(PathBuf::from("note.m4a"), "audio/mp4", Some(900));
        let options = TranscriptionOptions {
            language: None,
            prompt: None,
            model: Some("mock-custom".to_string()),
        };

        let transcript = tauri::async_runtime::block_on(service.transcribe(input, options))
            .expect("transcription service should return mock transcript");

        assert_eq!(service.provider_name(), MOCK_PROVIDER_NAME);
        assert_eq!(
            service.provider_capabilities(),
            ProviderCapabilities::new(true, true, false)
        );
        assert_eq!(transcript.text, "Mock transcript for note.m4a");
        assert_eq!(transcript.provider, MOCK_PROVIDER_NAME);
        assert_eq!(transcript.model.as_deref(), Some("mock-custom"));
        assert_eq!(transcript.language, None);
        assert_eq!(transcript.duration_ms, Some(900));
    }
}
