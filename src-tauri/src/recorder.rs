use crate::transcription::AudioInput;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig, SupportedStreamConfig,
};
use serde::Serialize;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender, TryRecvError},
    Arc, Mutex, MutexGuard,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const RECORDING_MIME_TYPE: &str = "audio/wav";
const WAV_BITS_PER_SAMPLE: u16 = 16;
const WAV_BYTES_PER_SAMPLE: u16 = WAV_BITS_PER_SAMPLE / 8;
const MAX_RECORDING_DURATION: Duration = Duration::from_secs(15 * 60);
const MAX_RECORDING_DURATION_MS: u64 = 15 * 60 * 1000;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecorderPhase {
    Idle,
    Starting,
    Recording,
    Stopping,
    Cancelling,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSnapshot {
    pub phase: RecorderPhase,
    pub active_session_id: Option<String>,
    pub input_device_name: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
    pub max_duration_ms: u64,
    pub limit_reached: bool,
    pub last_completed_session_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInputDevice {
    pub name: String,
    pub is_default: bool,
}

struct RecordingSession {
    id: String,
    started_at: Instant,
    max_duration: Duration,
    audio_path: PathBuf,
    input_device_name: String,
    sample_rate_hz: u32,
    channels: u16,
    runtime: Box<dyn RecordingRuntime>,
}

impl RecordingSession {
    fn new(id: String, started_capture: StartedRecordingCapture, max_duration: Duration) -> Self {
        Self {
            id,
            started_at: Instant::now(),
            max_duration,
            audio_path: started_capture.audio_path,
            input_device_name: started_capture.input_device_name,
            sample_rate_hz: started_capture.sample_rate_hz,
            channels: started_capture.channels,
            runtime: started_capture.runtime,
        }
    }

    fn active_session(&self) -> ActiveRecordingSession {
        ActiveRecordingSession {
            id: self.id.clone(),
            input_device_name: self.input_device_name.clone(),
        }
    }

    fn snapshot(&self, phase: RecorderPhase) -> RecorderSnapshot {
        RecorderSnapshot {
            phase,
            active_session_id: Some(self.id.clone()),
            input_device_name: Some(self.input_device_name.clone()),
            elapsed_ms: Some(self.elapsed_ms()),
            remaining_ms: Some(self.remaining_ms()),
            max_duration_ms: self.max_duration_ms(),
            limit_reached: false,
            last_completed_session_id: None,
        }
    }

    fn poll_completion(&mut self) -> Result<Option<FinishedRecordingArtifact>, RecorderError> {
        match self.runtime.poll_completion()? {
            Some(RuntimeCompletion::Stopped(artifact)) => Ok(Some(artifact)),
            Some(RuntimeCompletion::Cancelled) => Err(RecorderError::AudioStreamFailure {
                message: "recording thread returned an unexpected cancel result".to_string(),
            }),
            None => Ok(None),
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started_at
            .elapsed()
            .min(self.max_duration)
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    fn remaining_ms(&self) -> u64 {
        self.max_duration_ms().saturating_sub(self.elapsed_ms())
    }

    fn max_duration_ms(&self) -> u64 {
        self.max_duration.as_millis().try_into().unwrap_or(u64::MAX)
    }

    fn stop_result(self) -> Result<StoppedRecording, RecorderError> {
        let RecordingSession {
            id,
            started_at,
            max_duration,
            audio_path,
            input_device_name,
            sample_rate_hz,
            channels,
            runtime,
            ..
        } = self;
        let finished_artifact = runtime.finish()?;
        let duration_ms = if finished_artifact.limit_reached {
            max_duration.as_millis().try_into().unwrap_or(u64::MAX)
        } else {
            started_at
                .elapsed()
                .min(max_duration)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX)
        };

        Ok(Self::build_stopped_recording_from_parts(
            id,
            audio_path,
            input_device_name,
            sample_rate_hz,
            channels,
            duration_ms,
            finished_artifact,
        ))
    }

    fn build_stopped_recording(
        self,
        finished_artifact: FinishedRecordingArtifact,
    ) -> StoppedRecording {
        let duration_ms = if finished_artifact.limit_reached {
            self.max_duration_ms()
        } else {
            self.elapsed_ms()
        };
        let RecordingSession {
            id,
            audio_path,
            input_device_name,
            sample_rate_hz,
            channels,
            ..
        } = self;

        Self::build_stopped_recording_from_parts(
            id,
            audio_path,
            input_device_name,
            sample_rate_hz,
            channels,
            duration_ms,
            finished_artifact,
        )
    }

    fn build_stopped_recording_from_parts(
        id: String,
        audio_path: PathBuf,
        input_device_name: String,
        sample_rate_hz: u32,
        channels: u16,
        duration_ms: u64,
        finished_artifact: FinishedRecordingArtifact,
    ) -> StoppedRecording {
        StoppedRecording {
            session_id: id,
            audio_input: AudioInput::new(audio_path, RECORDING_MIME_TYPE, Some(duration_ms)),
            input_device_name,
            sample_rate_hz,
            channels,
            file_size_bytes: finished_artifact.file_size_bytes,
            limit_reached: finished_artifact.limit_reached,
        }
    }

    fn cancel_result(self) -> Result<CancelledRecording, RecorderError> {
        let cancelled_artifact = self.runtime.cancel()?;

        Ok(CancelledRecording {
            session_id: self.id,
            deleted_audio_path: cancelled_artifact.deleted_audio_path,
        })
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRecordingSession {
    pub id: String,
    pub input_device_name: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoppedRecording {
    pub session_id: String,
    pub audio_input: AudioInput,
    pub input_device_name: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub file_size_bytes: u64,
    pub limit_reached: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelledRecording {
    pub session_id: String,
    pub deleted_audio_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LastStoppedRecording {
    session_id: String,
    input_device_name: String,
    elapsed_ms: u64,
    max_duration_ms: u64,
    remaining_ms: u64,
    limit_reached: bool,
}

impl LastStoppedRecording {
    fn from_stopped(stopped: &StoppedRecording, max_duration_ms: u64) -> Self {
        let elapsed_ms = stopped.audio_input.duration_ms.unwrap_or_default();

        Self {
            session_id: stopped.session_id.clone(),
            input_device_name: stopped.input_device_name.clone(),
            elapsed_ms,
            max_duration_ms,
            remaining_ms: max_duration_ms.saturating_sub(elapsed_ms),
            limit_reached: stopped.limit_reached,
        }
    }
}

enum RecorderLifecycleState {
    Idle,
    Starting(RecordingSession),
    Recording(RecordingSession),
    Stopping(RecordingSession),
    Cancelling(RecordingSession),
}

impl RecorderLifecycleState {
    fn snapshot(&self) -> RecorderSnapshot {
        match self {
            Self::Idle => RecorderSnapshot {
                phase: RecorderPhase::Idle,
                active_session_id: None,
                input_device_name: None,
                elapsed_ms: None,
                remaining_ms: None,
                max_duration_ms: MAX_RECORDING_DURATION_MS,
                limit_reached: false,
                last_completed_session_id: None,
            },
            Self::Starting(session) => session.snapshot(RecorderPhase::Starting),
            Self::Recording(session) => session.snapshot(RecorderPhase::Recording),
            Self::Stopping(session) => session.snapshot(RecorderPhase::Stopping),
            Self::Cancelling(session) => session.snapshot(RecorderPhase::Cancelling),
        }
    }
}

struct RecorderLifecycle {
    state: RecorderLifecycleState,
    last_stopped_recording: Option<LastStoppedRecording>,
}

impl Default for RecorderLifecycle {
    fn default() -> Self {
        Self {
            state: RecorderLifecycleState::Idle,
            last_stopped_recording: None,
        }
    }
}

impl RecorderLifecycle {
    fn snapshot(&self) -> RecorderSnapshot {
        match &self.state {
            RecorderLifecycleState::Idle => {
                let mut snapshot = self.state.snapshot();
                if let Some(last_stopped_recording) = &self.last_stopped_recording {
                    snapshot.input_device_name =
                        Some(last_stopped_recording.input_device_name.clone());
                    snapshot.elapsed_ms = Some(last_stopped_recording.elapsed_ms);
                    snapshot.remaining_ms = Some(last_stopped_recording.remaining_ms);
                    snapshot.max_duration_ms = last_stopped_recording.max_duration_ms;
                    snapshot.limit_reached = last_stopped_recording.limit_reached;
                    snapshot.last_completed_session_id =
                        Some(last_stopped_recording.session_id.clone());
                }

                snapshot
            }
            _ => self.state.snapshot(),
        }
    }

    fn refresh_recording(&mut self) -> Result<Option<StoppedRecording>, RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Recording(mut session) => match session.poll_completion()? {
                Some(finished_artifact) => {
                    let max_duration_ms = session.max_duration_ms();
                    let stopped = session.build_stopped_recording(finished_artifact);
                    self.last_stopped_recording = Some(LastStoppedRecording::from_stopped(
                        &stopped,
                        max_duration_ms,
                    ));

                    Ok(Some(stopped))
                }
                None => {
                    self.state = RecorderLifecycleState::Recording(session);
                    Ok(None)
                }
            },
            state => {
                self.state = state;
                Ok(None)
            }
        }
    }

    fn begin_start(&mut self, session: RecordingSession) -> Result<(), RecorderError> {
        match self.state {
            RecorderLifecycleState::Idle => {
                self.last_stopped_recording = None;
                self.state = RecorderLifecycleState::Starting(session);
                Ok(())
            }
            _ => Err(RecorderError::TransitionRejected {
                attempted: RecorderTransition::Start,
                current_phase: self.state.snapshot().phase,
            }),
        }
    }

    fn complete_start(&mut self) -> Result<ActiveRecordingSession, RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Starting(session) => {
                let active_session = session.active_session();
                self.state = RecorderLifecycleState::Recording(session);
                Ok(active_session)
            }
            state => {
                let current_phase = state.snapshot().phase;
                self.state = state;
                Err(RecorderError::TransitionRejected {
                    attempted: RecorderTransition::Start,
                    current_phase,
                })
            }
        }
    }

    fn begin_stop(&mut self) -> Result<(), RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Recording(session) => {
                self.state = RecorderLifecycleState::Stopping(session);
                Ok(())
            }
            state => {
                let current_phase = state.snapshot().phase;
                self.state = state;
                Err(RecorderError::TransitionRejected {
                    attempted: RecorderTransition::Stop,
                    current_phase,
                })
            }
        }
    }

    fn complete_stop(&mut self) -> Result<StoppedRecording, RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Stopping(session) => {
                let max_duration_ms = session.max_duration_ms();
                let stopped = session.stop_result()?;
                self.last_stopped_recording = Some(LastStoppedRecording::from_stopped(
                    &stopped,
                    max_duration_ms,
                ));

                Ok(stopped)
            }
            state => {
                let current_phase = state.snapshot().phase;
                self.state = state;
                Err(RecorderError::TransitionRejected {
                    attempted: RecorderTransition::Stop,
                    current_phase,
                })
            }
        }
    }

    fn begin_cancel(&mut self) -> Result<(), RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Recording(session) => {
                self.state = RecorderLifecycleState::Cancelling(session);
                Ok(())
            }
            state => {
                let current_phase = state.snapshot().phase;
                self.state = state;
                Err(RecorderError::TransitionRejected {
                    attempted: RecorderTransition::Cancel,
                    current_phase,
                })
            }
        }
    }

    fn complete_cancel(&mut self) -> Result<CancelledRecording, RecorderError> {
        match std::mem::replace(&mut self.state, RecorderLifecycleState::Idle) {
            RecorderLifecycleState::Cancelling(session) => {
                self.last_stopped_recording = None;
                session.cancel_result()
            }
            state => {
                let current_phase = state.snapshot().phase;
                self.state = state;
                Err(RecorderError::TransitionRejected {
                    attempted: RecorderTransition::Cancel,
                    current_phase,
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderTransition {
    Start,
    Stop,
    Cancel,
}

impl RecorderTransition {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Cancel => "cancel",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderError {
    TransitionRejected {
        attempted: RecorderTransition,
        current_phase: RecorderPhase,
    },
    StateUnavailable,
    DefaultInputDeviceUnavailable,
    InputDeviceNotFound {
        name: String,
    },
    InputDeviceEnumerationFailed {
        message: String,
    },
    InputDeviceNameUnavailable {
        message: String,
    },
    InputDeviceConfigUnavailable {
        device_name: String,
        message: String,
    },
    RecordingDirectoryCreateFailed {
        path: PathBuf,
        message: String,
    },
    RecordingFileCreateFailed {
        path: PathBuf,
        message: String,
    },
    AudioStreamBuildFailed {
        device_name: String,
        message: String,
    },
    AudioStreamStartFailed {
        device_name: String,
        message: String,
    },
    AudioStreamFailure {
        message: String,
    },
    UnsupportedSampleFormat {
        format: String,
    },
    RecordingFinalizeFailed {
        path: PathBuf,
        message: String,
    },
    RecordingDeleteFailed {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for RecorderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransitionRejected {
                attempted,
                current_phase,
            } => write!(
                formatter,
                "cannot {} recorder while it is in the {:?} phase",
                attempted.name(),
                current_phase
            ),
            Self::StateUnavailable => formatter.write_str("recorder state is unavailable"),
            Self::DefaultInputDeviceUnavailable => {
                formatter.write_str("no default audio input device is available")
            }
            Self::InputDeviceNotFound { name } => {
                write!(formatter, "audio input device '{name}' was not found")
            }
            Self::InputDeviceEnumerationFailed { message } => {
                write!(
                    formatter,
                    "failed to enumerate audio input devices: {message}"
                )
            }
            Self::InputDeviceNameUnavailable { message } => {
                write!(
                    formatter,
                    "failed to read audio input device name: {message}"
                )
            }
            Self::InputDeviceConfigUnavailable {
                device_name,
                message,
            } => write!(
                formatter,
                "failed to read default input config for '{device_name}': {message}"
            ),
            Self::RecordingDirectoryCreateFailed { path, message } => write!(
                formatter,
                "failed to create recording directory '{}': {message}",
                path.display()
            ),
            Self::RecordingFileCreateFailed { path, message } => write!(
                formatter,
                "failed to create recording file '{}': {message}",
                path.display()
            ),
            Self::AudioStreamBuildFailed {
                device_name,
                message,
            } => write!(
                formatter,
                "failed to build audio input stream for '{device_name}': {message}"
            ),
            Self::AudioStreamStartFailed {
                device_name,
                message,
            } => write!(
                formatter,
                "failed to start audio input stream for '{device_name}': {message}"
            ),
            Self::AudioStreamFailure { message } => {
                write!(formatter, "audio input stream failed: {message}")
            }
            Self::UnsupportedSampleFormat { format } => {
                write!(formatter, "unsupported input sample format: {format}")
            }
            Self::RecordingFinalizeFailed { path, message } => write!(
                formatter,
                "failed to finalize recording file '{}': {message}",
                path.display()
            ),
            Self::RecordingDeleteFailed { path, message } => write!(
                formatter,
                "failed to delete recording file '{}': {message}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RecorderError {}

pub struct RecorderService {
    lifecycle: Mutex<RecorderLifecycle>,
    next_session_number: AtomicU64,
    recordings_dir: PathBuf,
    backend: Arc<dyn RecordingBackend>,
    max_recording_duration: Duration,
}

impl RecorderService {
    pub fn new(recordings_dir: PathBuf) -> Self {
        Self::with_backend_and_max_duration(
            recordings_dir,
            Arc::new(CpalRecordingBackend::new()),
            MAX_RECORDING_DURATION,
        )
    }

    fn with_backend_and_max_duration(
        recordings_dir: PathBuf,
        backend: Arc<dyn RecordingBackend>,
        max_recording_duration: Duration,
    ) -> Self {
        Self {
            lifecycle: Mutex::new(RecorderLifecycle::default()),
            next_session_number: AtomicU64::new(1),
            recordings_dir,
            backend,
            max_recording_duration,
        }
    }

    pub fn snapshot(&self) -> Result<RecorderSnapshot, RecorderError> {
        let mut lifecycle = self.lifecycle()?;
        let _ = lifecycle.refresh_recording()?;

        Ok(lifecycle.snapshot())
    }

    pub fn list_input_devices(&self) -> Result<Vec<RecordingInputDevice>, RecorderError> {
        self.backend.list_input_devices()
    }

    pub fn start(
        &self,
        selected_device_name: Option<String>,
    ) -> Result<ActiveRecordingSession, RecorderError> {
        let session_id = format!(
            "recording-{}",
            self.next_session_number.fetch_add(1, Ordering::Relaxed)
        );

        let mut lifecycle = self.lifecycle()?;
        let _ = lifecycle.refresh_recording()?;
        if lifecycle.snapshot().phase != RecorderPhase::Idle {
            return Err(RecorderError::TransitionRejected {
                attempted: RecorderTransition::Start,
                current_phase: lifecycle.snapshot().phase,
            });
        }

        let started_capture = self.backend.start_capture(StartRecordingRequest {
            session_id: session_id.clone(),
            selected_device_name,
            recordings_dir: self.recordings_dir.clone(),
            max_duration: self.max_recording_duration,
        })?;
        let session =
            RecordingSession::new(session_id, started_capture, self.max_recording_duration);

        lifecycle.begin_start(session)?;
        lifecycle.complete_start()
    }

    pub fn stop(&self) -> Result<StoppedRecording, RecorderError> {
        let mut lifecycle = self.lifecycle()?;
        if let Some(stopped) = lifecycle.refresh_recording()? {
            return Ok(stopped);
        }

        lifecycle.begin_stop()?;
        lifecycle.complete_stop()
    }

    pub fn cancel(&self) -> Result<CancelledRecording, RecorderError> {
        let mut lifecycle = self.lifecycle()?;
        let _ = lifecycle.refresh_recording()?;

        lifecycle.begin_cancel()?;
        lifecycle.complete_cancel()
    }

    fn lifecycle(&self) -> Result<MutexGuard<'_, RecorderLifecycle>, RecorderError> {
        self.lifecycle
            .lock()
            .map_err(|_| RecorderError::StateUnavailable)
    }
}

struct StartRecordingRequest {
    session_id: String,
    selected_device_name: Option<String>,
    recordings_dir: PathBuf,
    max_duration: Duration,
}

struct StartedRecordingCapture {
    audio_path: PathBuf,
    input_device_name: String,
    sample_rate_hz: u32,
    channels: u16,
    runtime: Box<dyn RecordingRuntime>,
}

struct FinishedRecordingArtifact {
    file_size_bytes: u64,
    limit_reached: bool,
}

struct CancelledRecordingArtifact {
    deleted_audio_path: Option<PathBuf>,
}

trait RecordingRuntime: Send {
    fn finish(self: Box<Self>) -> Result<FinishedRecordingArtifact, RecorderError>;
    fn cancel(self: Box<Self>) -> Result<CancelledRecordingArtifact, RecorderError>;
    fn poll_completion(&mut self) -> Result<Option<RuntimeCompletion>, RecorderError>;
}

enum RuntimeCompletion {
    Stopped(FinishedRecordingArtifact),
    Cancelled,
}

trait RecordingBackend: Send + Sync {
    fn list_input_devices(&self) -> Result<Vec<RecordingInputDevice>, RecorderError>;
    fn start_capture(
        &self,
        request: StartRecordingRequest,
    ) -> Result<StartedRecordingCapture, RecorderError>;
}

#[derive(Default)]
struct CpalRecordingBackend;

impl CpalRecordingBackend {
    fn new() -> Self {
        Self
    }

    fn host(&self) -> cpal::Host {
        cpal::default_host()
    }

    fn build_capture(
        &self,
        request: StartRecordingRequest,
    ) -> Result<StartedRecordingCapture, RecorderError> {
        let host = self.host();
        let selected_device = select_input_device(&host, request.selected_device_name.as_deref())?;
        let input_device_name = read_device_name(&selected_device)?;
        let supported_config = selected_device.default_input_config().map_err(|error| {
            RecorderError::InputDeviceConfigUnavailable {
                device_name: input_device_name.clone(),
                message: error.to_string(),
            }
        })?;
        let sample_rate_hz = supported_config.sample_rate().0;
        let channels = supported_config.channels();

        fs::create_dir_all(&request.recordings_dir).map_err(|error| {
            RecorderError::RecordingDirectoryCreateFailed {
                path: request.recordings_dir.clone(),
                message: error.to_string(),
            }
        })?;

        let audio_path = request
            .recordings_dir
            .join(format!("{}.wav", request.session_id));
        let (control_sender, control_receiver) = mpsc::sync_channel(1);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let thread_audio_path = audio_path.clone();
        let thread_device_name = input_device_name.clone();
        let join_handle = thread::Builder::new()
            .name(format!("recorder-{}", request.session_id))
            .spawn(move || {
                run_capture_thread(
                    thread_audio_path,
                    thread_device_name,
                    request.max_duration,
                    startup_sender,
                    control_receiver,
                    result_sender,
                );
            })
            .map_err(|error| RecorderError::AudioStreamStartFailed {
                device_name: input_device_name.clone(),
                message: error.to_string(),
            })?;

        match startup_receiver.recv() {
            Ok(Ok(())) => Ok(StartedRecordingCapture {
                audio_path,
                input_device_name,
                sample_rate_hz,
                channels,
                runtime: Box::new(CpalRecordingRuntime {
                    control_sender,
                    result_receiver,
                    join_handle: Some(join_handle),
                }),
            }),
            Ok(Err(error)) => {
                let _ = join_handle.join();
                Err(error)
            }
            Err(_) => {
                let _ = join_handle.join();
                Err(RecorderError::AudioStreamStartFailed {
                    device_name: input_device_name,
                    message: "recording thread did not report startup status".to_string(),
                })
            }
        }
    }
}

impl RecordingBackend for CpalRecordingBackend {
    fn list_input_devices(&self) -> Result<Vec<RecordingInputDevice>, RecorderError> {
        let host = self.host();
        let default_name = host
            .default_input_device()
            .map(|device| read_device_name(&device))
            .transpose()?;
        let devices = host
            .input_devices()
            .map_err(|error| RecorderError::InputDeviceEnumerationFailed {
                message: error.to_string(),
            })?
            .map(|device| {
                let name = read_device_name(&device)?;
                Ok(RecordingInputDevice {
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    name,
                })
            })
            .collect::<Result<Vec<_>, RecorderError>>()?;

        Ok(devices)
    }

    fn start_capture(
        &self,
        request: StartRecordingRequest,
    ) -> Result<StartedRecordingCapture, RecorderError> {
        self.build_capture(request)
    }
}

struct CpalRecordingRuntime {
    control_sender: SyncSender<CaptureThreadCommand>,
    result_receiver: Receiver<Result<CaptureThreadResult, RecorderError>>,
    join_handle: Option<JoinHandle<()>>,
}

enum CaptureThreadCommand {
    Stop,
    Cancel,
}

enum CaptureThreadResult {
    Stopped(FinishedRecordingArtifact),
    Cancelled(CancelledRecordingArtifact),
}

impl CpalRecordingRuntime {
    fn finish_inner(&mut self) -> Result<FinishedRecordingArtifact, RecorderError> {
        if let Some(result) = self.try_receive_ready_result()? {
            match result {
                CaptureThreadResult::Stopped(artifact) => Ok(artifact),
                CaptureThreadResult::Cancelled(_) => Err(RecorderError::AudioStreamFailure {
                    message: "recording thread returned an unexpected cancel result".to_string(),
                }),
            }
        } else {
            match self.control_sender.send(CaptureThreadCommand::Stop) {
                Ok(()) => {}
                Err(_) => return self.receive_result_after_shutdown(),
            }

            match self.receive_result()? {
                CaptureThreadResult::Stopped(artifact) => Ok(artifact),
                CaptureThreadResult::Cancelled(_) => Err(RecorderError::AudioStreamFailure {
                    message: "recording thread returned an unexpected cancel result".to_string(),
                }),
            }
        }
    }

    fn cancel_inner(&mut self) -> Result<CancelledRecordingArtifact, RecorderError> {
        if let Some(result) = self.try_receive_ready_result()? {
            match result {
                CaptureThreadResult::Cancelled(artifact) => Ok(artifact),
                CaptureThreadResult::Stopped(_) => Err(RecorderError::AudioStreamFailure {
                    message: "recording thread returned an unexpected stop result".to_string(),
                }),
            }
        } else {
            self.control_sender
                .send(CaptureThreadCommand::Cancel)
                .map_err(|_| RecorderError::AudioStreamFailure {
                    message: "recording thread is unavailable".to_string(),
                })?;

            match self.receive_result()? {
                CaptureThreadResult::Cancelled(artifact) => Ok(artifact),
                CaptureThreadResult::Stopped(_) => Err(RecorderError::AudioStreamFailure {
                    message: "recording thread returned an unexpected stop result".to_string(),
                }),
            }
        }
    }

    fn join_thread(&mut self) -> Result<(), RecorderError> {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle
                .join()
                .map_err(|_| RecorderError::AudioStreamFailure {
                    message: "recording thread panicked".to_string(),
                })?;
        }

        Ok(())
    }

    fn try_receive_ready_result(&mut self) -> Result<Option<CaptureThreadResult>, RecorderError> {
        match self.result_receiver.try_recv() {
            Ok(result) => {
                self.join_thread()?;
                result.map(Some)
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.join_thread()?;
                Err(RecorderError::AudioStreamFailure {
                    message: "recording thread stopped before returning a result".to_string(),
                })
            }
        }
    }

    fn receive_result(&mut self) -> Result<CaptureThreadResult, RecorderError> {
        let result =
            self.result_receiver
                .recv()
                .map_err(|_| RecorderError::AudioStreamFailure {
                    message: "recording thread stopped before returning a result".to_string(),
                })?;
        self.join_thread()?;

        result
    }

    fn receive_result_after_shutdown(
        &mut self,
    ) -> Result<FinishedRecordingArtifact, RecorderError> {
        match self.receive_result()? {
            CaptureThreadResult::Stopped(artifact) => Ok(artifact),
            CaptureThreadResult::Cancelled(_) => Err(RecorderError::AudioStreamFailure {
                message: "recording thread returned an unexpected cancel result".to_string(),
            }),
        }
    }
}

impl RecordingRuntime for CpalRecordingRuntime {
    fn finish(mut self: Box<Self>) -> Result<FinishedRecordingArtifact, RecorderError> {
        self.finish_inner()
    }

    fn cancel(mut self: Box<Self>) -> Result<CancelledRecordingArtifact, RecorderError> {
        self.cancel_inner()
    }

    fn poll_completion(&mut self) -> Result<Option<RuntimeCompletion>, RecorderError> {
        self.try_receive_ready_result().map(|result| {
            result.map(|value| match value {
                CaptureThreadResult::Stopped(artifact) => RuntimeCompletion::Stopped(artifact),
                CaptureThreadResult::Cancelled(_) => RuntimeCompletion::Cancelled,
            })
        })
    }
}

struct CaptureSharedState {
    writer: Option<WavFileWriter>,
    stream_error: Option<String>,
}

impl CaptureSharedState {
    fn new(writer: WavFileWriter) -> Self {
        Self {
            writer: Some(writer),
            stream_error: None,
        }
    }
}

fn run_capture_thread(
    audio_path: PathBuf,
    input_device_name: String,
    max_duration: Duration,
    startup_sender: SyncSender<Result<(), RecorderError>>,
    control_receiver: Receiver<CaptureThreadCommand>,
    result_sender: SyncSender<Result<CaptureThreadResult, RecorderError>>,
) {
    let setup_result = (|| -> Result<(Stream, Arc<Mutex<CaptureSharedState>>), RecorderError> {
        let host = cpal::default_host();
        let device = select_input_device(&host, Some(input_device_name.as_str()))?;
        let supported_config = device.default_input_config().map_err(|error| {
            RecorderError::InputDeviceConfigUnavailable {
                device_name: input_device_name.clone(),
                message: error.to_string(),
            }
        })?;
        let writer = WavFileWriter::create(
            &audio_path,
            supported_config.sample_rate().0,
            supported_config.channels(),
        )?;
        let shared_state = Arc::new(Mutex::new(CaptureSharedState::new(writer)));
        let stream = build_input_stream(
            &device,
            &supported_config,
            shared_state.clone(),
            &input_device_name,
        )?;
        stream
            .play()
            .map_err(|error| RecorderError::AudioStreamStartFailed {
                device_name: input_device_name.clone(),
                message: error.to_string(),
            })?;

        Ok((stream, shared_state))
    })();

    let (stream, shared_state) = match setup_result {
        Ok(value) => value,
        Err(error) => {
            let _ = startup_sender.send(Err(error));
            return;
        }
    };

    if startup_sender.send(Ok(())).is_err() {
        drop(stream);
        let _ = cancel_capture_file(&audio_path, &shared_state);
        return;
    }

    let (command, limit_reached) = match control_receiver.recv_timeout(max_duration) {
        Ok(command) => (command, false),
        Err(mpsc::RecvTimeoutError::Timeout) => (CaptureThreadCommand::Stop, true),
        Err(mpsc::RecvTimeoutError::Disconnected) => (CaptureThreadCommand::Cancel, false),
    };
    drop(stream);

    let result = match command {
        CaptureThreadCommand::Stop => {
            finalize_capture_file(&audio_path, &shared_state, limit_reached)
                .map(CaptureThreadResult::Stopped)
        }
        CaptureThreadCommand::Cancel => {
            cancel_capture_file(&audio_path, &shared_state).map(CaptureThreadResult::Cancelled)
        }
    };

    let _ = result_sender.send(result);
}

fn finalize_capture_file(
    audio_path: &Path,
    shared_state: &Arc<Mutex<CaptureSharedState>>,
    limit_reached: bool,
) -> Result<FinishedRecordingArtifact, RecorderError> {
    let mut capture_state = shared_state
        .lock()
        .map_err(|_| RecorderError::StateUnavailable)?;
    let stream_error = capture_state.stream_error.take();
    let writer =
        capture_state
            .writer
            .take()
            .ok_or_else(|| RecorderError::RecordingFinalizeFailed {
                path: audio_path.to_path_buf(),
                message: "recording writer is unavailable".to_string(),
            })?;
    drop(capture_state);

    let file_size_bytes = writer.finalize(audio_path)?;

    if let Some(message) = stream_error {
        let _ = fs::remove_file(audio_path);
        return Err(RecorderError::AudioStreamFailure { message });
    }

    Ok(FinishedRecordingArtifact {
        file_size_bytes,
        limit_reached,
    })
}

fn cancel_capture_file(
    audio_path: &Path,
    shared_state: &Arc<Mutex<CaptureSharedState>>,
) -> Result<CancelledRecordingArtifact, RecorderError> {
    let mut capture_state = shared_state
        .lock()
        .map_err(|_| RecorderError::StateUnavailable)?;
    capture_state.writer.take();
    drop(capture_state);

    match fs::remove_file(audio_path) {
        Ok(()) => Ok(CancelledRecordingArtifact {
            deleted_audio_path: Some(audio_path.to_path_buf()),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(CancelledRecordingArtifact {
            deleted_audio_path: None,
        }),
        Err(error) => Err(RecorderError::RecordingDeleteFailed {
            path: audio_path.to_path_buf(),
            message: error.to_string(),
        }),
    }
}

fn select_input_device(
    host: &cpal::Host,
    selected_device_name: Option<&str>,
) -> Result<cpal::Device, RecorderError> {
    if let Some(selected_device_name) = selected_device_name {
        let trimmed_device_name = selected_device_name.trim();
        if trimmed_device_name.is_empty() {
            return host
                .default_input_device()
                .ok_or(RecorderError::DefaultInputDeviceUnavailable);
        }

        return host
            .input_devices()
            .map_err(|error| RecorderError::InputDeviceEnumerationFailed {
                message: error.to_string(),
            })?
            .find_map(|device| match read_device_name(&device) {
                Ok(device_name) if device_name == trimmed_device_name => Some(Ok(device)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .transpose()?
            .ok_or_else(|| RecorderError::InputDeviceNotFound {
                name: trimmed_device_name.to_string(),
            });
    }

    host.default_input_device()
        .ok_or(RecorderError::DefaultInputDeviceUnavailable)
}

fn read_device_name(device: &cpal::Device) -> Result<String, RecorderError> {
    device
        .name()
        .map_err(|error| RecorderError::InputDeviceNameUnavailable {
            message: error.to_string(),
        })
}

fn build_input_stream(
    device: &cpal::Device,
    supported_config: &SupportedStreamConfig,
    shared_state: Arc<Mutex<CaptureSharedState>>,
    device_name: &str,
) -> Result<Stream, RecorderError> {
    let stream_config: StreamConfig = supported_config.clone().into();

    match supported_config.sample_format() {
        SampleFormat::I8 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: i8| {
                i16::from(sample) << 8
            })
        }
        SampleFormat::I16 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: i16| sample)
        }
        SampleFormat::I32 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: i32| {
                (sample >> 16) as i16
            })
        }
        SampleFormat::I64 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: i64| {
                (sample >> 48) as i16
            })
        }
        SampleFormat::U8 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: u8| {
                (i16::from(sample) - 128) << 8
            })
        }
        SampleFormat::U16 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: u16| {
                (i32::from(sample) - 32_768) as i16
            })
        }
        SampleFormat::U32 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: u32| {
                ((sample >> 16) as i32 - 32_768) as i16
            })
        }
        SampleFormat::U64 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: u64| {
                ((sample >> 48) as i64 - 32_768) as i16
            })
        }
        SampleFormat::F32 => {
            build_typed_input_stream(device, &stream_config, shared_state, |sample: f32| {
                float_sample_to_i16(sample as f64)
            })
        }
        SampleFormat::F64 => {
            build_typed_input_stream(device, &stream_config, shared_state, float_sample_to_i16)
        }
        sample_format => Err(RecorderError::UnsupportedSampleFormat {
            format: format!("{sample_format:?}"),
        }),
    }
    .map_err(|error| match error {
        RecorderError::UnsupportedSampleFormat { .. } => error,
        other => RecorderError::AudioStreamBuildFailed {
            device_name: device_name.to_string(),
            message: other.to_string(),
        },
    })
}

fn build_typed_input_stream<T, F>(
    device: &cpal::Device,
    stream_config: &StreamConfig,
    shared_state: Arc<Mutex<CaptureSharedState>>,
    convert_sample: F,
) -> Result<Stream, RecorderError>
where
    T: cpal::SizedSample,
    F: Fn(T) -> i16 + Send + Sync + Copy + 'static,
{
    let error_state = shared_state.clone();
    device
        .build_input_stream(
            stream_config,
            move |data: &[T], _| {
                write_input_samples(data, &shared_state, convert_sample);
            },
            move |error| {
                if let Ok(mut capture_state) = error_state.lock() {
                    if capture_state.stream_error.is_none() {
                        capture_state.stream_error = Some(error.to_string());
                    }
                }
            },
            None,
        )
        .map_err(|error| RecorderError::AudioStreamFailure {
            message: error.to_string(),
        })
}

fn write_input_samples<T, F>(
    data: &[T],
    shared_state: &Arc<Mutex<CaptureSharedState>>,
    convert_sample: F,
) where
    T: Copy,
    F: Fn(T) -> i16,
{
    if let Ok(mut capture_state) = shared_state.lock() {
        if capture_state.stream_error.is_some() {
            return;
        }

        let Some(writer) = capture_state.writer.as_mut() else {
            return;
        };

        let pcm_samples = data.iter().copied().map(convert_sample).collect::<Vec<_>>();
        if let Err(error) = writer.write_samples(&pcm_samples) {
            capture_state.stream_error = Some(error.to_string());
        }
    }
}

fn float_sample_to_i16(sample: f64) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * f64::from(i16::MAX)).round() as i16
}

struct WavFileWriter {
    file: File,
    sample_rate_hz: u32,
    channels: u16,
    data_bytes_written: u32,
}

impl WavFileWriter {
    fn create(path: &Path, sample_rate_hz: u32, channels: u16) -> Result<Self, RecorderError> {
        let mut file =
            File::create(path).map_err(|error| RecorderError::RecordingFileCreateFailed {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        write_wav_header(&mut file, sample_rate_hz, channels, 0).map_err(|error| {
            RecorderError::RecordingFileCreateFailed {
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?;

        Ok(Self {
            file,
            sample_rate_hz,
            channels,
            data_bytes_written: 0,
        })
    }

    fn write_samples(&mut self, samples: &[i16]) -> io::Result<()> {
        for sample in samples {
            self.file.write_all(&sample.to_le_bytes())?;
        }

        let additional_bytes = u32::try_from(samples.len())
            .ok()
            .and_then(|sample_count| sample_count.checked_mul(u32::from(WAV_BYTES_PER_SAMPLE)))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "recording exceeds WAV size limits",
                )
            })?;
        self.data_bytes_written = self
            .data_bytes_written
            .checked_add(additional_bytes)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "recording exceeds WAV size limits",
                )
            })?;

        Ok(())
    }

    fn finalize(mut self, path: &Path) -> Result<u64, RecorderError> {
        self.file.seek(SeekFrom::Start(0)).map_err(|error| {
            RecorderError::RecordingFinalizeFailed {
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?;
        write_wav_header(
            &mut self.file,
            self.sample_rate_hz,
            self.channels,
            self.data_bytes_written,
        )
        .and_then(|()| self.file.flush())
        .map_err(|error| RecorderError::RecordingFinalizeFailed {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        self.file
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|error| RecorderError::RecordingFinalizeFailed {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}

fn write_wav_header(
    file: &mut File,
    sample_rate_hz: u32,
    channels: u16,
    data_bytes_written: u32,
) -> io::Result<()> {
    let byte_rate = sample_rate_hz
        .checked_mul(u32::from(channels))
        .and_then(|value| value.checked_mul(u32::from(WAV_BYTES_PER_SAMPLE)))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid WAV byte rate"))?;
    let block_align = channels.checked_mul(WAV_BYTES_PER_SAMPLE).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid WAV block alignment")
    })?;
    let chunk_size = 36u32
        .checked_add(data_bytes_written)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid WAV chunk size"))?;

    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate_hz.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&WAV_BITS_PER_SAMPLE.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_bytes_written.to_le_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ActiveRecordingSession, CancelledRecording, RecorderError, RecorderLifecycle,
        RecorderPhase, RecorderService, RecorderTransition, RecordingInputDevice, RecordingRuntime,
        RecordingSession, RuntimeCompletion, StartedRecordingCapture, StoppedRecording,
        MAX_RECORDING_DURATION, MAX_RECORDING_DURATION_MS,
    };
    use crate::transcription::AudioInput;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    static NEXT_TEST_DIRECTORY_NUMBER: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let directory_number = NEXT_TEST_DIRECTORY_NUMBER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::current_dir()
                .expect("current dir should be available")
                .join("target")
                .join("test-artifacts")
                .join(format!("{name}-{directory_number}"));
            fs::create_dir_all(&path).expect("test directory should be created");

            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[derive(Clone, Default)]
    struct FakeRecordingBackend {
        devices: Vec<RecordingInputDevice>,
        create_file_on_start: bool,
        limit_reached_on_poll: bool,
    }

    impl FakeRecordingBackend {
        fn with_devices(devices: Vec<RecordingInputDevice>) -> Self {
            Self {
                devices,
                create_file_on_start: true,
                limit_reached_on_poll: false,
            }
        }
    }

    impl super::RecordingBackend for FakeRecordingBackend {
        fn list_input_devices(&self) -> Result<Vec<RecordingInputDevice>, RecorderError> {
            Ok(self.devices.clone())
        }

        fn start_capture(
            &self,
            request: super::StartRecordingRequest,
        ) -> Result<StartedRecordingCapture, RecorderError> {
            let input_device_name = request
                .selected_device_name
                .unwrap_or_else(|| "Default Test Microphone".to_string());
            let audio_path = request
                .recordings_dir
                .join(format!("{}.wav", request.session_id));

            fs::create_dir_all(&request.recordings_dir).expect("recording directory should exist");
            if self.create_file_on_start {
                let mut file = File::create(&audio_path).expect("recording file should be created");
                file.write_all(b"test-audio")
                    .expect("recording file should be written");
            }

            Ok(StartedRecordingCapture {
                audio_path: audio_path.clone(),
                input_device_name,
                sample_rate_hz: 16_000,
                channels: 1,
                runtime: Box::new(FakeRecordingRuntime {
                    audio_path,
                    file_size_bytes: 9,
                    limit_reached_on_poll: self.limit_reached_on_poll,
                    completion_reported: false,
                }),
            })
        }
    }

    struct FakeRecordingRuntime {
        audio_path: PathBuf,
        file_size_bytes: u64,
        limit_reached_on_poll: bool,
        completion_reported: bool,
    }

    impl RecordingRuntime for FakeRecordingRuntime {
        fn finish(self: Box<Self>) -> Result<super::FinishedRecordingArtifact, RecorderError> {
            Ok(super::FinishedRecordingArtifact {
                file_size_bytes: self.file_size_bytes,
                limit_reached: false,
            })
        }

        fn cancel(self: Box<Self>) -> Result<super::CancelledRecordingArtifact, RecorderError> {
            if self.audio_path.exists() {
                fs::remove_file(&self.audio_path).expect("recording file should be removed");
                Ok(super::CancelledRecordingArtifact {
                    deleted_audio_path: Some(self.audio_path.clone()),
                })
            } else {
                Ok(super::CancelledRecordingArtifact {
                    deleted_audio_path: None,
                })
            }
        }

        fn poll_completion(&mut self) -> Result<Option<RuntimeCompletion>, RecorderError> {
            if self.limit_reached_on_poll && !self.completion_reported {
                self.completion_reported = true;

                return Ok(Some(RuntimeCompletion::Stopped(
                    super::FinishedRecordingArtifact {
                        file_size_bytes: self.file_size_bytes,
                        limit_reached: true,
                    },
                )));
            }

            Ok(None)
        }
    }

    fn recorder_service_for_tests(test_name: &str) -> (RecorderService, TestDirectory) {
        recorder_service_for_tests_with_backend(
            test_name,
            FakeRecordingBackend::with_devices(vec![RecordingInputDevice {
                name: "Default Test Microphone".to_string(),
                is_default: true,
            }]),
            MAX_RECORDING_DURATION,
        )
    }

    fn recorder_service_for_tests_with_backend(
        test_name: &str,
        backend: FakeRecordingBackend,
        max_recording_duration: std::time::Duration,
    ) -> (RecorderService, TestDirectory) {
        let test_directory = TestDirectory::new(test_name);
        let service = RecorderService::with_backend_and_max_duration(
            test_directory.path.clone(),
            Arc::new(backend),
            max_recording_duration,
        );

        (service, test_directory)
    }

    #[test]
    fn recorder_service_lists_input_devices() {
        let (service, _test_directory) = recorder_service_for_tests("list-input-devices");

        let devices = service
            .list_input_devices()
            .expect("device listing should succeed");

        assert_eq!(
            devices,
            vec![RecordingInputDevice {
                name: "Default Test Microphone".to_string(),
                is_default: true,
            }]
        );
    }

    #[test]
    fn recorder_service_starts_and_reports_recording_state() {
        let (service, _test_directory) = recorder_service_for_tests("start-state");

        let session = service.start(None).expect("recorder should start");
        let snapshot = service.snapshot().expect("snapshot should succeed");

        assert_eq!(session.id, "recording-1");
        assert_eq!(session.input_device_name, "Default Test Microphone");
        assert_eq!(snapshot.phase, RecorderPhase::Recording);
        assert_eq!(snapshot.active_session_id.as_deref(), Some("recording-1"));
        assert_eq!(
            snapshot.input_device_name.as_deref(),
            Some("Default Test Microphone")
        );
        assert_eq!(snapshot.max_duration_ms, MAX_RECORDING_DURATION_MS);
        assert!(!snapshot.limit_reached);
        assert_eq!(snapshot.last_completed_session_id, None);
        assert!(snapshot.elapsed_ms.is_some());
        assert!(snapshot.remaining_ms.is_some());
    }

    #[test]
    fn recorder_service_rejects_second_start_while_recording() {
        let (service, _test_directory) = recorder_service_for_tests("reject-second-start");

        service.start(None).expect("first start should succeed");
        let error = service.start(None).expect_err("second start should fail");

        assert_eq!(
            error,
            RecorderError::TransitionRejected {
                attempted: RecorderTransition::Start,
                current_phase: RecorderPhase::Recording,
            }
        );
    }

    #[test]
    fn recorder_service_stop_returns_recording_metadata_and_keeps_audio_file() {
        let (service, test_directory) = recorder_service_for_tests("stop-recording");

        let session = service.start(None).expect("recorder should start");
        let stopped = service.stop().expect("stop should succeed");
        let snapshot = service.snapshot().expect("snapshot should succeed");

        assert_eq!(stopped.session_id, session.id);
        assert_eq!(stopped.audio_input.mime_type, "audio/wav");
        assert_eq!(stopped.input_device_name, "Default Test Microphone");
        assert_eq!(stopped.sample_rate_hz, 16_000);
        assert_eq!(stopped.channels, 1);
        assert_eq!(stopped.file_size_bytes, 9);
        assert!(!stopped.limit_reached);
        assert!(stopped.audio_input.path.exists());
        assert!(stopped
            .audio_input
            .path
            .starts_with(test_directory.path.as_path()));
        assert_eq!(snapshot.phase, RecorderPhase::Idle);
        assert_eq!(snapshot.active_session_id, None);
        assert_eq!(
            snapshot.last_completed_session_id.as_deref(),
            Some(session.id.as_str())
        );
        assert!(!snapshot.limit_reached);
    }

    #[test]
    fn recorder_service_cancel_deletes_audio_file_and_returns_to_idle() {
        let (service, _test_directory) = recorder_service_for_tests("cancel-recording");

        let session = service.start(None).expect("recorder should start");
        let audio_path = service.recordings_dir.join(format!("{}.wav", session.id));
        let cancelled = service.cancel().expect("cancel should succeed");
        let snapshot = service.snapshot().expect("snapshot should succeed");

        assert_eq!(cancelled.session_id, session.id);
        assert_eq!(cancelled.deleted_audio_path.as_ref(), Some(&audio_path));
        assert!(!audio_path.exists());
        assert_eq!(snapshot.phase, RecorderPhase::Idle);
        assert_eq!(snapshot.active_session_id, None);
        assert_eq!(snapshot.last_completed_session_id, None);
        assert!(!snapshot.limit_reached);
    }

    #[test]
    fn recorder_service_snapshot_reports_limit_status_after_auto_stop() {
        let backend = FakeRecordingBackend {
            limit_reached_on_poll: true,
            ..FakeRecordingBackend::with_devices(vec![RecordingInputDevice {
                name: "Default Test Microphone".to_string(),
                is_default: true,
            }])
        };
        let (service, _test_directory) = recorder_service_for_tests_with_backend(
            "limit-status",
            backend,
            std::time::Duration::from_millis(250),
        );

        let session = service.start(None).expect("recorder should start");
        let snapshot = service.snapshot().expect("snapshot should succeed");

        assert_eq!(snapshot.phase, RecorderPhase::Idle);
        assert_eq!(snapshot.active_session_id, None);
        assert_eq!(
            snapshot.input_device_name.as_deref(),
            Some("Default Test Microphone")
        );
        assert_eq!(snapshot.elapsed_ms, Some(250));
        assert_eq!(snapshot.remaining_ms, Some(0));
        assert_eq!(snapshot.max_duration_ms, 250);
        assert!(snapshot.limit_reached);
        assert_eq!(
            snapshot.last_completed_session_id.as_deref(),
            Some(session.id.as_str())
        );
    }

    #[test]
    fn recorder_service_stop_returns_auto_stopped_result_when_limit_was_reached() {
        let backend = FakeRecordingBackend {
            limit_reached_on_poll: true,
            ..FakeRecordingBackend::with_devices(vec![RecordingInputDevice {
                name: "Default Test Microphone".to_string(),
                is_default: true,
            }])
        };
        let (service, _test_directory) = recorder_service_for_tests_with_backend(
            "limit-stop",
            backend,
            std::time::Duration::from_millis(250),
        );

        let session = service.start(None).expect("recorder should start");
        let stopped = service.stop().expect("stop should return auto-stop result");

        assert_eq!(stopped.session_id, session.id);
        assert_eq!(stopped.audio_input.duration_ms, Some(250));
        assert!(stopped.limit_reached);
        assert!(stopped.audio_input.path.exists());
    }

    #[test]
    fn lifecycle_scaffolds_start_stop_and_cancel_phases_explicitly() {
        let test_directory = TestDirectory::new("lifecycle-scaffold");
        let mut lifecycle = RecorderLifecycle::default();

        lifecycle
            .begin_start(build_recording_session(
                "recording-42",
                test_directory.path.as_path(),
                "Lifecycle Microphone",
            ))
            .expect("begin start should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Starting);

        lifecycle
            .complete_start()
            .expect("complete start should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Recording);

        lifecycle.begin_stop().expect("begin stop should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Stopping);

        lifecycle
            .complete_stop()
            .expect("complete stop should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Idle);

        lifecycle
            .begin_start(build_recording_session(
                "recording-43",
                test_directory.path.as_path(),
                "Lifecycle Microphone",
            ))
            .expect("begin start should succeed");
        lifecycle
            .complete_start()
            .expect("complete start should succeed");

        lifecycle
            .begin_cancel()
            .expect("begin cancel should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Cancelling);

        lifecycle
            .complete_cancel()
            .expect("complete cancel should succeed");
        assert_eq!(lifecycle.snapshot().phase, RecorderPhase::Idle);
    }

    #[test]
    fn lifecycle_rejects_stop_without_active_recording() {
        let mut lifecycle = RecorderLifecycle::default();

        let error = lifecycle.begin_stop().expect_err("stop should fail");

        assert_eq!(
            error,
            RecorderError::TransitionRejected {
                attempted: RecorderTransition::Stop,
                current_phase: RecorderPhase::Idle,
            }
        );
    }

    #[test]
    fn wav_file_writer_persists_pcm_header_and_sample_bytes() {
        let test_directory = TestDirectory::new("wav-writer");
        let audio_path = test_directory.path.join("recording.wav");

        let mut writer =
            super::WavFileWriter::create(&audio_path, 8_000, 1).expect("writer should be created");
        writer
            .write_samples(&[0, i16::MAX, i16::MIN])
            .expect("samples should be written");
        let file_size = writer
            .finalize(&audio_path)
            .expect("writer should finalize");
        let bytes = fs::read(&audio_path).expect("audio file should be readable");

        assert_eq!(file_size, bytes.len() as u64);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
        assert_eq!(bytes.len(), 44 + 6);
    }

    fn build_recording_session(
        id: &str,
        recordings_dir: &Path,
        input_device_name: &str,
    ) -> RecordingSession {
        let audio_path = recordings_dir.join(format!("{id}.wav"));
        let mut file = File::create(&audio_path).expect("session recording file should be created");
        file.write_all(b"audio")
            .expect("session recording file should be written");

        RecordingSession::new(
            id.to_string(),
            StartedRecordingCapture {
                audio_path,
                input_device_name: input_device_name.to_string(),
                sample_rate_hz: 44_100,
                channels: 1,
                runtime: Box::new(FakeRecordingRuntime {
                    audio_path: recordings_dir.join(format!("{id}.wav")),
                    file_size_bytes: 5,
                    limit_reached_on_poll: false,
                    completion_reported: false,
                }),
            },
            MAX_RECORDING_DURATION,
        )
    }

    #[allow(dead_code)]
    fn _assert_result_types(
        _active: ActiveRecordingSession,
        _stopped: StoppedRecording,
        _cancelled: CancelledRecording,
        _audio_input: AudioInput,
    ) {
    }
}
