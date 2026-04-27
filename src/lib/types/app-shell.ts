import type { AppSettings, ProviderId } from "$lib/settings/schema";

export type AppSection = "recording" | "history" | "settings";

export type RecordingPhase = "idle" | "recording" | "transcribing" | "completed" | "error";

export interface NavigationSection {
  id: AppSection;
  label: string;
  blurb: string;
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

export type SettingsDraft = AppSettings;

export type { ProviderId };
