import type { AppSettings, ProviderId } from "$lib/settings/schema";

export type AppSection = "recording" | "history" | "settings";

export type RecordingPhase = "idle" | "recording" | "transcribing" | "completed" | "error";

export interface NavigationSection {
  id: AppSection;
  label: string;
  blurb: string;
}

export interface RecordingTiming {
  elapsedMs: number | null;
  remainingMs: number | null;
  maxDurationMs: number | null;
  limitReached: boolean;
}

export interface RecordedAudioMetadata {
  sessionId: string;
  path: string;
  mimeType: string;
  durationMs: number | null;
  inputDeviceName: string;
  sampleRateHz: number;
  channels: number;
  fileSizeBytes: number;
  limitReached: boolean;
  maxDurationMs: number | null;
}

export interface AppStatus {
  phase: RecordingPhase;
  phaseLabel: string;
  headline: string;
  detail: string;
  transcriptTitle: string;
  transcriptPreview: string;
  inputLabel: string;
  durationLabel: string;
  recordingTiming: RecordingTiming | null;
  recordedAudio: RecordedAudioMetadata | null;
}

export interface ProviderOption {
  id: ProviderId;
  label: string;
  blurb: string;
  note: string;
}

export interface RecordingInputOption {
  value: string;
  label: string;
  isDefault: boolean;
  unavailable?: boolean;
}

export type GeminiApiKeyPresenceState = "loading" | "present" | "missing" | "error";

export type GeminiApiKeyActionState = "idle" | "checking" | "saving" | "clearing" | "error";

export type SettingsDraft = AppSettings;

export type { ProviderId };
