# Implementation Plan: Local-First Desktop Transcription App

## 1. Project Overview

This project is a local-first desktop application for fast audio transcription. The user can start recording from the app UI, tray icon, or global shortcut. After recording, the app sends the audio to a selected transcription provider, receives the transcription, saves it locally, copies it to the clipboard, and displays it in the app history.

The MVP uses Gemini as the first transcription provider to reduce initial complexity and avoid requiring users to install local AI models. The architecture is designed to support fully local transcription later through `whisper.cpp` as a sidecar binary.

The recommended stack is:

```text
Desktop shell: Tauri v2
Frontend: Svelte + TypeScript
Native layer: Rust
Audio capture: cpal
Remote transcription: Gemini API
Local storage: SQLite
Settings storage: Tauri Store
Secret storage: Stronghold or OS keychain
Clipboard: Tauri/Rust clipboard integration
Tray icon: Tauri tray support
Global shortcut: tauri-plugin-global-shortcut
Future local transcription: whisper.cpp sidecar
```

The app must not require an external backend. The Rust layer is bundled into the desktop binary and acts as the native core for system-level operations.

---

## 2. Product Goals

The MVP should allow the user to:

1. Open a polished desktop app.
2. Configure a Gemini API key.
3. Start and stop audio recording.
4. Send the recorded audio to Gemini for transcription.
5. Receive plain transcription text.
6. Save the transcription to local history.
7. Automatically copy the transcription to the clipboard.
8. View and reuse previous transcriptions.
9. Use a tray icon and global shortcut for faster access.

The MVP should avoid:

1. Real-time streaming transcription.
2. Mandatory local model installation.
3. Cloud sync.
4. User accounts.
5. Custom external backend.
6. Auto-paste into active input.
7. Rust/C++ FFI for local transcription.
8. Complex provider management beyond Gemini.

---

## 3. Core Architecture

```text
Tauri Desktop App
├─ Frontend: Svelte + TypeScript
│  ├─ Main recording UI
│  ├─ Transcription history
│  ├─ Settings screen
│  ├─ Provider selection
│  └─ User feedback and error display
│
├─ Rust Core
│  ├─ AudioRecorder
│  ├─ TranscriptionService
│  ├─ TranscriptionProvider abstraction
│  ├─ GeminiProvider
│  ├─ HistoryRepository
│  ├─ SettingsStore
│  ├─ SecretStore
│  ├─ ClipboardService
│  ├─ TrayService
│  └─ ShortcutService
│
├─ Local SQLite Database
│  └─ Transcription history
│
└─ Future Sidecar
   └─ whisper.cpp for local/offline transcription
```

The frontend should remain focused on UI and user interaction. The Rust core should own native concerns such as audio recording, clipboard access, local storage, secrets, provider execution, shortcuts, and tray behavior.

---

## 4. Core Data Model

Initial SQLite table:

```sql
CREATE TABLE transcriptions (
  id TEXT PRIMARY KEY,
  text TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT,
  language TEXT,
  duration_ms INTEGER,
  audio_path TEXT,
  audio_deleted INTEGER NOT NULL DEFAULT 1,
  copied_to_clipboard INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_transcriptions_created_at
ON transcriptions(created_at DESC);

CREATE INDEX idx_transcriptions_provider
ON transcriptions(provider);
```

Settings stored in Tauri Store:

```text
default_provider
default_language
shortcut
auto_copy
save_audio_files
save_transcription_history
selected_microphone
```

Secrets stored in Stronghold or OS keychain:

```text
Gemini API key
future provider API keys
```

Recommended defaults:

```text
Auto-copy: enabled
Auto-paste: disabled / unavailable in MVP
Save audio files: disabled
Save transcription history: enabled
Default provider: Gemini
Default language: auto-detect
```

---

## 5. Provider Abstraction

Create the provider abstraction before implementing Gemini. This keeps the MVP compatible with future local transcription.

Suggested Rust shape:

```rust
#[async_trait::async_trait]
pub trait TranscriptionProvider: Send + Sync {
    async fn transcribe(
        &self,
        input: AudioInput,
        options: TranscriptionOptions,
    ) -> anyhow::Result<Transcript>;

    fn name(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;
}
```

Core types:

```rust
pub struct AudioInput {
    pub path: PathBuf,
    pub mime_type: String,
    pub duration_ms: Option<u64>,
}

pub struct TranscriptionOptions {
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
}

pub struct Transcript {
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<u64>,
}

pub struct ProviderCapabilities {
    pub supports_language_hint: bool,
    pub supports_local_execution: bool,
    pub supports_timestamps: bool,
}
```

The first implementation should be `GeminiProvider`. A future implementation should be `LocalWhisperProvider` using a sidecar binary.

---

## 6. Release Strategy

The MVP should be delivered through small internal releases. Each release should produce a testable desktop app, even if incomplete.

The goal is to reduce risk by proving one slice at a time:

1. Desktop shell.
2. Local state and storage.
3. Audio recording.
4. Mock transcription.
5. Real Gemini transcription.
6. Clipboard and history.
7. Tray and shortcut.
8. Security hardening.
9. MVP packaging.

---

# Implementation Releases

## Release 0.1 — Project Bootstrap

### Goal

Create the base Tauri desktop app with Svelte, TypeScript, Rust, formatting, linting, and project structure.

### Scope

* Initialize Tauri v2 app.
* Add Svelte + TypeScript frontend.
* Configure Rust workspace or module layout.
* Add basic app window.
* Add development scripts.
* Add formatting and linting.
* Add basic frontend routing or view structure.

### Deliverables

* App launches locally in development mode.
* Main window renders a basic home screen.
* Project has clean folder structure.
* CI or local scripts can run formatting and checks.

### Acceptance Criteria

* `npm run dev` or equivalent starts the desktop app.
* Frontend can call a simple Rust command such as `ping`.
* Rust command returns a test response to the frontend.

---

## Release 0.2 — UI Skeleton and App State

### Goal

Create the first version of the UI without real native behavior.

### Scope

## Release 0.3 — Settings Storage

## Release 0.4 — SQLite History Foundation

## Release 0.5 — Mock Transcription Pipeline

## Release 0.6 — Audio Recording Foundation

## Release 0.7 — Recording UX and Safety Limits

## Release 0.8 — Secrets Storage for Gemini API Key

## Release 0.9 — Gemini Provider Integration

## Release 1.0-alpha — Full MVP Flow Without Tray/Shortcut

## Release 1.1-alpha — Clipboard and History UX

## Release 1.2-alpha — Tray Icon

## Release 1.3-alpha — Global Shortcut

## Release 1.4-alpha — Notifications and Status Feedback

## Release 1.5-beta — Error Handling and Resilience

## Release 1.6-beta — Privacy and Security Review

## Release 1.7-beta — Packaging and Installer

## Release 1.8-beta — Cross-Platform Testing

## Release 1.9-rc — MVP Polish

## Release 2.0 — MVP Release

## Release 2.1 — Local Whisper Foundation

## Release 2.2 — Model Management

## Release 2.3 — Local Transcription

## Release 2.4 — Optional Auto-Paste

## Release 2.5 — Advanced History

## Reliability First

## Privacy by Default

## Small Rust Surface Area

## Provider Flexibility

## Platform Reality
