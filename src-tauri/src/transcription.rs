use crate::secret_store::SecretStoreService;
use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{fmt, fs, path::PathBuf, sync::Arc};

const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const GEMINI_UPLOAD_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const GEMINI_API_KEY_HEADER: &str = "x-goog-api-key";
const GEMINI_UPLOAD_URL_HEADER: &str = "x-goog-upload-url";
const GEMINI_UPLOAD_PROTOCOL_HEADER: &str = "X-Goog-Upload-Protocol";
const GEMINI_UPLOAD_COMMAND_HEADER: &str = "X-Goog-Upload-Command";
const GEMINI_UPLOAD_OFFSET_HEADER: &str = "X-Goog-Upload-Offset";
const GEMINI_UPLOAD_CONTENT_LENGTH_HEADER: &str = "X-Goog-Upload-Header-Content-Length";
const GEMINI_UPLOAD_CONTENT_TYPE_HEADER: &str = "X-Goog-Upload-Header-Content-Type";
const DEFAULT_GEMINI_PROMPT: &str = "Transcribe the provided audio as plain text. Return only the transcription without summaries, speaker-label guesses, or extra commentary.";

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

pub const GEMINI_PROVIDER_NAME: &str = "gemini";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";

#[derive(Clone)]
pub struct GeminiProvider {
    api_key_provider: Arc<dyn GeminiApiKeyProvider>,
    api_client: Arc<dyn GeminiApiClient>,
}

impl GeminiProvider {
    pub fn new(secret_store_service: SecretStoreService) -> Self {
        Self::with_dependencies(
            Arc::new(secret_store_service),
            Arc::new(ReqwestGeminiApiClient::new()),
        )
    }

    fn with_dependencies(
        api_key_provider: Arc<dyn GeminiApiKeyProvider>,
        api_client: Arc<dyn GeminiApiClient>,
    ) -> Self {
        Self {
            api_key_provider,
            api_client,
        }
    }
}

#[async_trait::async_trait]
impl TranscriptionProvider for GeminiProvider {
    async fn transcribe(
        &self,
        input: AudioInput,
        options: TranscriptionOptions,
    ) -> Result<Transcript, TranscriptionProviderError> {
        let resolved_options = ResolvedGeminiOptions::from_options(options);
        let api_key = self.api_key_provider.read_gemini_api_key()?;
        let uploaded_file = self.api_client.upload_file(&api_key, &input).await?;
        let transcript_result = self
            .api_client
            .generate_content(&api_key, &uploaded_file, &resolved_options)
            .await;
        let cleanup_result = self
            .api_client
            .delete_file(&api_key, &uploaded_file.name)
            .await;

        let text = match (transcript_result, cleanup_result) {
            (Ok(text), Ok(())) => text,
            (Ok(_), Err(cleanup_error)) => {
                return Err(TranscriptionProviderError::new(format!(
                    "Gemini transcription cleanup failed: {cleanup_error}"
                )))
            }
            (Err(transcription_error), Ok(())) => return Err(transcription_error),
            (Err(transcription_error), Err(cleanup_error)) => {
                return Err(TranscriptionProviderError::new(format!(
                    "{transcription_error}; uploaded Gemini file cleanup also failed: {cleanup_error}"
                )))
            }
        };

        Ok(Transcript {
            text,
            provider: self.name().to_string(),
            model: Some(resolved_options.model),
            language: resolved_options.language,
            duration_ms: input.duration_ms,
        })
    }

    fn name(&self) -> &'static str {
        GEMINI_PROVIDER_NAME
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::new(true, false, false)
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

trait GeminiApiKeyProvider: Send + Sync {
    fn read_gemini_api_key(&self) -> Result<String, TranscriptionProviderError>;
}

impl GeminiApiKeyProvider for SecretStoreService {
    fn read_gemini_api_key(&self) -> Result<String, TranscriptionProviderError> {
        self.read_gemini_api_key()
            .map_err(TranscriptionProviderError::new)
    }
}

#[async_trait::async_trait]
trait GeminiApiClient: Send + Sync {
    async fn upload_file(
        &self,
        api_key: &str,
        input: &AudioInput,
    ) -> Result<GeminiUploadedFile, TranscriptionProviderError>;

    async fn generate_content(
        &self,
        api_key: &str,
        uploaded_file: &GeminiUploadedFile,
        options: &ResolvedGeminiOptions,
    ) -> Result<String, TranscriptionProviderError>;

    async fn delete_file(
        &self,
        api_key: &str,
        file_name: &str,
    ) -> Result<(), TranscriptionProviderError>;
}

#[derive(Clone, Debug)]
struct ResolvedGeminiOptions {
    language: Option<String>,
    prompt: Option<String>,
    model: String,
}

impl ResolvedGeminiOptions {
    fn from_options(options: TranscriptionOptions) -> Self {
        Self {
            language: normalize_optional_text(options.language),
            prompt: normalize_optional_text(options.prompt),
            model: normalize_optional_text(options.model)
                .unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string()),
        }
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GeminiUploadedFile {
    name: String,
    uri: String,
    mime_type: String,
}

#[derive(Clone, Debug)]
struct ReqwestGeminiApiClient {
    http_client: Client,
}

impl ReqwestGeminiApiClient {
    fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl GeminiApiClient for ReqwestGeminiApiClient {
    async fn upload_file(
        &self,
        api_key: &str,
        input: &AudioInput,
    ) -> Result<GeminiUploadedFile, TranscriptionProviderError> {
        let audio_bytes = fs::read(&input.path).map_err(|error| {
            TranscriptionProviderError::new(format!(
                "failed to read audio input {}: {error}",
                input.path.display()
            ))
        })?;
        let display_name = input
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("audio-input")
            .to_string();
        let start_response = self
            .http_client
            .post(format!(
                "{}/upload/v1beta/files",
                GEMINI_UPLOAD_BASE_URL.trim_end_matches('/')
            ))
            .header(GEMINI_API_KEY_HEADER, api_key)
            .header(GEMINI_UPLOAD_PROTOCOL_HEADER, "resumable")
            .header(GEMINI_UPLOAD_COMMAND_HEADER, "start")
            .header(
                GEMINI_UPLOAD_CONTENT_LENGTH_HEADER,
                audio_bytes.len().to_string(),
            )
            .header(GEMINI_UPLOAD_CONTENT_TYPE_HEADER, input.mime_type.as_str())
            .header(CONTENT_TYPE, "application/json")
            .json(&GeminiUploadRequest {
                file: GeminiUploadMetadata { display_name },
            })
            .send()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!(
                    "Gemini upload initialization failed: {error}"
                ))
            })?;
        let start_response =
            ensure_success(start_response, "Gemini upload initialization failed").await?;
        let upload_url = start_response
            .headers()
            .get(GEMINI_UPLOAD_URL_HEADER)
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                TranscriptionProviderError::new(
                    "Gemini upload initialization did not return an upload URL",
                )
            })?
            .to_string();
        let upload_response = self
            .http_client
            .post(upload_url)
            .header(CONTENT_LENGTH, audio_bytes.len().to_string())
            .header(GEMINI_UPLOAD_OFFSET_HEADER, "0")
            .header(GEMINI_UPLOAD_COMMAND_HEADER, "upload, finalize")
            .body(audio_bytes)
            .send()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!("Gemini file upload failed: {error}"))
            })?;
        let upload_response = ensure_success(upload_response, "Gemini file upload failed").await?;
        let payload = upload_response
            .json::<GeminiUploadResponse>()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!(
                    "Gemini file upload response was invalid: {error}"
                ))
            })?;

        Ok(GeminiUploadedFile {
            name: payload.file.name,
            uri: payload.file.uri,
            mime_type: payload
                .file
                .mime_type
                .unwrap_or_else(|| input.mime_type.clone()),
        })
    }

    async fn generate_content(
        &self,
        api_key: &str,
        uploaded_file: &GeminiUploadedFile,
        options: &ResolvedGeminiOptions,
    ) -> Result<String, TranscriptionProviderError> {
        let response = self
            .http_client
            .post(format!(
                "{}/v1beta/models/{}:generateContent",
                GEMINI_API_BASE_URL.trim_end_matches('/'),
                options.model
            ))
            .header(GEMINI_API_KEY_HEADER, api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(&GeminiGenerateContentRequest {
                contents: vec![GeminiRequestContent {
                    parts: vec![
                        GeminiRequestPart::Text {
                            text: build_gemini_prompt(options),
                        },
                        GeminiRequestPart::FileData {
                            file_data: GeminiFileData {
                                mime_type: uploaded_file.mime_type.clone(),
                                file_uri: uploaded_file.uri.clone(),
                            },
                        },
                    ],
                }],
            })
            .send()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!(
                    "Gemini transcription request failed: {error}"
                ))
            })?;
        let response = ensure_success(response, "Gemini transcription request failed").await?;
        let payload = response
            .json::<GeminiGenerateContentResponse>()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!(
                    "Gemini transcription response was invalid: {error}"
                ))
            })?;

        extract_gemini_transcript_text(&payload).ok_or_else(|| {
            TranscriptionProviderError::new(
                "Gemini transcription response did not contain transcript text",
            )
        })
    }

    async fn delete_file(
        &self,
        api_key: &str,
        file_name: &str,
    ) -> Result<(), TranscriptionProviderError> {
        let response = self
            .http_client
            .delete(format!(
                "{}/v1beta/{}",
                GEMINI_API_BASE_URL.trim_end_matches('/'),
                file_name
            ))
            .header(GEMINI_API_KEY_HEADER, api_key)
            .send()
            .await
            .map_err(|error| {
                TranscriptionProviderError::new(format!(
                    "Gemini uploaded-file cleanup failed: {error}"
                ))
            })?;
        ensure_success(response, "Gemini uploaded-file cleanup failed")
            .await
            .map(|_| ())
    }
}

fn build_gemini_prompt(options: &ResolvedGeminiOptions) -> String {
    let mut segments = vec![DEFAULT_GEMINI_PROMPT.to_string()];

    if let Some(language) = options.language.as_deref() {
        segments.push(format!("Language hint: {language}."));
    }

    if let Some(prompt) = options.prompt.as_deref() {
        segments.push(format!("Additional instruction: {prompt}"));
    }

    segments.join("\n\n")
}

fn extract_gemini_transcript_text(response: &GeminiGenerateContentResponse) -> Option<String> {
    let text_segments = response
        .candidates
        .iter()
        .filter_map(|candidate| candidate.content.as_ref())
        .flat_map(|content| content.parts.iter())
        .filter_map(|part| part.text.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (!text_segments.is_empty()).then(|| text_segments.join("\n"))
}

async fn ensure_success(
    response: reqwest::Response,
    context: &str,
) -> Result<reqwest::Response, TranscriptionProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let response_body = response.text().await.unwrap_or_default();
    let details = format_error_details(&response_body);

    Err(TranscriptionProviderError::new(format!(
        "{context}: Gemini API returned {status}{details}"
    )))
}

fn format_error_details(response_body: &str) -> String {
    let normalized = response_body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return String::new();
    }

    let mut truncated = normalized.chars().take(240).collect::<String>();
    if normalized.chars().count() > 240 {
        truncated.push('…');
    }

    format!(" ({truncated})")
}

#[derive(Debug, Serialize)]
struct GeminiUploadRequest {
    file: GeminiUploadMetadata,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUploadMetadata {
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct GeminiUploadResponse {
    file: GeminiUploadResource,
}

#[derive(Debug, Deserialize)]
struct GeminiUploadResource {
    name: String,
    uri: String,
    #[serde(default, rename = "mimeType", alias = "mime_type")]
    mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerateContentRequest {
    contents: Vec<GeminiRequestContent>,
}

#[derive(Debug, Serialize)]
struct GeminiRequestContent {
    parts: Vec<GeminiRequestPart>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GeminiRequestPart {
    Text { text: String },
    FileData {
        #[serde(rename = "fileData")]
        file_data: GeminiFileData,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateContentResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    #[serde(default)]
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        build_gemini_prompt, extract_gemini_transcript_text, AudioInput, GeminiApiClient,
        GeminiApiKeyProvider, GeminiFileData, GeminiGenerateContentRequest,
        GeminiGenerateContentResponse, GeminiProvider, GeminiRequestContent, GeminiRequestPart,
        GeminiResponseContent, GeminiResponsePart, GeminiUploadMetadata, GeminiUploadRequest,
        GeminiUploadedFile, ProviderCapabilities, ResolvedGeminiOptions, Transcript,
        TranscriptionOptions, TranscriptionProvider, TranscriptionProviderError,
        TranscriptionService, DEFAULT_GEMINI_MODEL, GEMINI_PROVIDER_NAME,
    };
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

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
    fn gemini_provider_exposes_expected_metadata() {
        let provider = GeminiProvider::with_dependencies(
            Arc::new(FakeGeminiApiKeyProvider::with_key("gemini-secret-token")),
            Arc::new(FakeGeminiApiClient::default()),
        );

        assert_eq!(provider.name(), GEMINI_PROVIDER_NAME);
        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities::new(true, false, false)
        );
    }

    #[test]
    fn gemini_provider_transcribes_audio_and_cleans_up_uploaded_file() {
        let api_key_provider = Arc::new(FakeGeminiApiKeyProvider::with_key("gemini-secret-token"));
        let api_client = Arc::new(FakeGeminiApiClient {
            upload_result: Mutex::new(Ok(GeminiUploadedFile {
                name: "files/uploaded-audio".to_string(),
                uri: "https://example.test/files/uploaded-audio".to_string(),
                mime_type: "audio/wav".to_string(),
            })),
            generate_result: Mutex::new(Ok("First line\nSecond line".to_string())),
            delete_result: Mutex::new(Ok(())),
            ..Default::default()
        });
        let provider =
            GeminiProvider::with_dependencies(api_key_provider.clone(), api_client.clone());
        let input = AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210));
        let options = TranscriptionOptions {
            language: Some("  en-US  ".to_string()),
            prompt: Some("  keep punctuation  ".to_string()),
            model: None,
        };

        let transcript =
            tauri::async_runtime::block_on(provider.transcribe(input.clone(), options))
                .expect("Gemini provider should succeed");
        let generate_calls = api_client
            .generate_calls
            .lock()
            .expect("generate_calls lock should succeed")
            .clone();
        let delete_calls = api_client
            .delete_calls
            .lock()
            .expect("delete_calls lock should succeed")
            .clone();

        assert_eq!(api_key_provider.call_count.load(Ordering::Relaxed), 1);
        assert_eq!(transcript.text, "First line\nSecond line");
        assert_eq!(transcript.provider, GEMINI_PROVIDER_NAME);
        assert_eq!(transcript.model.as_deref(), Some(DEFAULT_GEMINI_MODEL));
        assert_eq!(transcript.language.as_deref(), Some("en-US"));
        assert_eq!(transcript.duration_ms, Some(3210));
        assert_eq!(generate_calls.len(), 1);
        assert_eq!(generate_calls[0].api_key, "gemini-secret-token");
        assert_eq!(generate_calls[0].model, DEFAULT_GEMINI_MODEL);
        assert!(generate_calls[0].prompt.contains("Language hint: en-US."));
        assert!(generate_calls[0]
            .prompt
            .contains("Additional instruction: keep punctuation"));
        assert_eq!(
            delete_calls,
            vec![(
                "gemini-secret-token".to_string(),
                "files/uploaded-audio".to_string()
            )]
        );
        assert_eq!(
            api_client
                .upload_calls
                .lock()
                .expect("upload_calls lock should succeed")
                .as_slice(),
            &[("gemini-secret-token".to_string(), input)]
        );
    }

    #[test]
    fn gemini_upload_request_serializes_metadata_in_camel_case() {
        let payload = serde_json::to_value(GeminiUploadRequest {
            file: GeminiUploadMetadata {
                display_name: "recording.wav".to_string(),
            },
        })
        .expect("upload request should serialize");

        assert_eq!(
            payload,
            json!({
                "file": {
                    "displayName": "recording.wav"
                }
            })
        );
    }

    #[test]
    fn gemini_generate_content_request_serializes_file_data_in_camel_case() {
        let payload = serde_json::to_value(GeminiGenerateContentRequest {
            contents: vec![GeminiRequestContent {
                parts: vec![
                    GeminiRequestPart::Text {
                        text: build_gemini_prompt(&ResolvedGeminiOptions::from_options(
                            TranscriptionOptions::default(),
                        )),
                    },
                    GeminiRequestPart::FileData {
                        file_data: GeminiFileData {
                            mime_type: "audio/wav".to_string(),
                            file_uri: "https://example.test/files/uploaded-audio".to_string(),
                        },
                    },
                ],
            }],
        })
        .expect("generate content request should serialize");

        assert_eq!(
            payload,
            json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": "Transcribe the provided audio as plain text. Return only the transcription without summaries, speaker-label guesses, or extra commentary."
                            },
                            {
                                "fileData": {
                                    "mimeType": "audio/wav",
                                    "fileUri": "https://example.test/files/uploaded-audio"
                                }
                            }
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn gemini_provider_still_cleans_up_when_generation_fails() {
        let api_client = Arc::new(FakeGeminiApiClient {
            upload_result: Mutex::new(Ok(GeminiUploadedFile {
                name: "files/uploaded-audio".to_string(),
                uri: "https://example.test/files/uploaded-audio".to_string(),
                mime_type: "audio/wav".to_string(),
            })),
            generate_result: Mutex::new(Err(TranscriptionProviderError::new(
                "Gemini transcription request failed",
            ))),
            delete_result: Mutex::new(Ok(())),
            ..Default::default()
        });
        let provider = GeminiProvider::with_dependencies(
            Arc::new(FakeGeminiApiKeyProvider::with_key("gemini-secret-token")),
            api_client.clone(),
        );

        let error = tauri::async_runtime::block_on(provider.transcribe(
            AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210)),
            TranscriptionOptions::default(),
        ))
        .expect_err("Gemini provider should return the transcription failure");

        assert_eq!(error.to_string(), "Gemini transcription request failed");
        assert_eq!(
            api_client
                .delete_calls
                .lock()
                .expect("delete_calls lock should succeed")
                .as_slice(),
            &[(
                "gemini-secret-token".to_string(),
                "files/uploaded-audio".to_string()
            )]
        );
    }

    #[test]
    fn gemini_provider_returns_cleanup_errors_after_successful_transcription() {
        let provider = GeminiProvider::with_dependencies(
            Arc::new(FakeGeminiApiKeyProvider::with_key("gemini-secret-token")),
            Arc::new(FakeGeminiApiClient {
                upload_result: Mutex::new(Ok(GeminiUploadedFile {
                    name: "files/uploaded-audio".to_string(),
                    uri: "https://example.test/files/uploaded-audio".to_string(),
                    mime_type: "audio/wav".to_string(),
                })),
                generate_result: Mutex::new(Ok("Transcript text".to_string())),
                delete_result: Mutex::new(Err(TranscriptionProviderError::new(
                    "Gemini uploaded-file cleanup failed",
                ))),
                ..Default::default()
            }),
        );

        let error = tauri::async_runtime::block_on(provider.transcribe(
            AudioInput::new(PathBuf::from("recording.wav"), "audio/wav", Some(3210)),
            TranscriptionOptions::default(),
        ))
        .expect_err("cleanup failures should surface");

        assert_eq!(
            error.to_string(),
            "Gemini transcription cleanup failed: Gemini uploaded-file cleanup failed"
        );
    }

    #[test]
    fn extract_gemini_transcript_text_collects_non_empty_text_parts() {
        let response = GeminiGenerateContentResponse {
            candidates: vec![super::GeminiCandidate {
                content: Some(GeminiResponseContent {
                    parts: vec![
                        GeminiResponsePart {
                            text: Some(" First line ".to_string()),
                        },
                        GeminiResponsePart { text: None },
                        GeminiResponsePart {
                            text: Some("Second line".to_string()),
                        },
                    ],
                }),
            }],
        };

        assert_eq!(
            extract_gemini_transcript_text(&response),
            Some("First line\nSecond line".to_string())
        );
    }

    #[test]
    fn transcription_service_delegates_to_provider() {
        let service = TranscriptionService::new(Arc::new(FakeTranscriptionProvider));
        let input = AudioInput::new(PathBuf::from("note.m4a"), "audio/mp4", Some(900));
        let options = TranscriptionOptions {
            language: None,
            prompt: None,
            model: Some("delegated-custom".to_string()),
        };

        let transcript = tauri::async_runtime::block_on(service.transcribe(input, options))
            .expect("transcription service should delegate to the provider");

        assert_eq!(service.provider_name(), "test-provider");
        assert_eq!(
            service.provider_capabilities(),
            ProviderCapabilities::new(true, false, true)
        );
        assert_eq!(transcript.text, "Delegated transcript for note.m4a");
        assert_eq!(transcript.provider, "test-provider");
        assert_eq!(transcript.model.as_deref(), Some("delegated-custom"));
        assert_eq!(transcript.language, None);
        assert_eq!(transcript.duration_ms, Some(900));
    }

    struct FakeTranscriptionProvider;

    #[async_trait::async_trait]
    impl TranscriptionProvider for FakeTranscriptionProvider {
        async fn transcribe(
            &self,
            input: AudioInput,
            options: TranscriptionOptions,
        ) -> Result<Transcript, TranscriptionProviderError> {
            let source_name = input
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("audio input");

            Ok(Transcript {
                text: format!("Delegated transcript for {source_name}"),
                provider: self.name().to_string(),
                model: options.model,
                language: options.language,
                duration_ms: input.duration_ms,
            })
        }

        fn name(&self) -> &'static str {
            "test-provider"
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::new(true, false, true)
        }
    }

    #[derive(Default)]
    struct FakeGeminiApiKeyProvider {
        api_key: Mutex<Option<String>>,
        error: Mutex<Option<TranscriptionProviderError>>,
        call_count: AtomicUsize,
    }

    impl FakeGeminiApiKeyProvider {
        fn with_key(api_key: &str) -> Self {
            Self {
                api_key: Mutex::new(Some(api_key.to_string())),
                error: Mutex::new(None),
                call_count: AtomicUsize::new(0),
            }
        }
    }

    impl GeminiApiKeyProvider for FakeGeminiApiKeyProvider {
        fn read_gemini_api_key(&self) -> Result<String, TranscriptionProviderError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            if let Some(error) = self
                .error
                .lock()
                .expect("error lock should succeed")
                .clone()
            {
                return Err(error);
            }

            self.api_key
                .lock()
                .expect("api_key lock should succeed")
                .clone()
                .ok_or_else(|| TranscriptionProviderError::new("Gemini API key is not configured"))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct FakeGenerateCall {
        api_key: String,
        uploaded_file: GeminiUploadedFile,
        model: String,
        prompt: String,
    }

    struct FakeGeminiApiClient {
        upload_result: Mutex<Result<GeminiUploadedFile, TranscriptionProviderError>>,
        generate_result: Mutex<Result<String, TranscriptionProviderError>>,
        delete_result: Mutex<Result<(), TranscriptionProviderError>>,
        upload_calls: Mutex<Vec<(String, AudioInput)>>,
        generate_calls: Mutex<Vec<FakeGenerateCall>>,
        delete_calls: Mutex<Vec<(String, String)>>,
    }

    impl Default for FakeGeminiApiClient {
        fn default() -> Self {
            Self {
                upload_result: Mutex::new(Err(TranscriptionProviderError::new(
                    "upload result was not configured",
                ))),
                generate_result: Mutex::new(Err(TranscriptionProviderError::new(
                    "generate result was not configured",
                ))),
                delete_result: Mutex::new(Err(TranscriptionProviderError::new(
                    "delete result was not configured",
                ))),
                upload_calls: Mutex::new(Vec::new()),
                generate_calls: Mutex::new(Vec::new()),
                delete_calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl GeminiApiClient for FakeGeminiApiClient {
        async fn upload_file(
            &self,
            api_key: &str,
            input: &AudioInput,
        ) -> Result<GeminiUploadedFile, TranscriptionProviderError> {
            self.upload_calls
                .lock()
                .expect("upload_calls lock should succeed")
                .push((api_key.to_string(), input.clone()));
            self.upload_result
                .lock()
                .expect("upload_result lock should succeed")
                .clone()
        }

        async fn generate_content(
            &self,
            api_key: &str,
            uploaded_file: &GeminiUploadedFile,
            options: &ResolvedGeminiOptions,
        ) -> Result<String, TranscriptionProviderError> {
            self.generate_calls
                .lock()
                .expect("generate_calls lock should succeed")
                .push(FakeGenerateCall {
                    api_key: api_key.to_string(),
                    uploaded_file: uploaded_file.clone(),
                    model: options.model.clone(),
                    prompt: super::build_gemini_prompt(options),
                });
            self.generate_result
                .lock()
                .expect("generate_result lock should succeed")
                .clone()
        }

        async fn delete_file(
            &self,
            api_key: &str,
            file_name: &str,
        ) -> Result<(), TranscriptionProviderError> {
            self.delete_calls
                .lock()
                .expect("delete_calls lock should succeed")
                .push((api_key.to_string(), file_name.to_string()));
            self.delete_result
                .lock()
                .expect("delete_result lock should succeed")
                .clone()
        }
    }
}
