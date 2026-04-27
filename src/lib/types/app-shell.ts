import type { AppSettings, ProviderId } from "$lib/settings/schema";

export type AppSection = "recording" | "history" | "settings";

export type RecordingPhase = "idle" | "recording" | "transcribing" | "completed" | "error";

export interface NavigationSection {
  id: AppSection;
  label: string;
  blurb: string;
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
}

export interface ProviderOption {
  id: ProviderId;
  label: string;
  blurb: string;
  note: string;
}

export type SettingsDraft = AppSettings;

export type { ProviderId };
