export const providerIds = ["gemini"] as const;

export type ProviderId = (typeof providerIds)[number];

export interface AppSettings {
  provider: ProviderId;
  defaultLanguage: string;
  shortcut: string | null;
  autoCopy: boolean;
  saveAudioFiles: boolean;
  saveTranscriptionHistory: boolean;
  selectedMicrophone: string;
}

export interface StoredAppSettings {
  default_provider: ProviderId;
  default_language: string;
  shortcut: string | null;
  auto_copy: boolean;
  save_audio_files: boolean;
  save_transcription_history: boolean;
  selected_microphone: string;
}

export const appSettingsStorePath = "settings.json";

export const appSettingsStoreKeys = {
  provider: "default_provider",
  defaultLanguage: "default_language",
  shortcut: "shortcut",
  autoCopy: "auto_copy",
  saveAudioFiles: "save_audio_files",
  saveTranscriptionHistory: "save_transcription_history",
  selectedMicrophone: "selected_microphone"
} as const;

export type AppSettingsStoreKey = (typeof appSettingsStoreKeys)[keyof typeof appSettingsStoreKeys];

export const appSettingsStoreKeyList = Object.values(appSettingsStoreKeys);

export const defaultAppSettings: AppSettings = Object.freeze({
  provider: "gemini",
  defaultLanguage: "auto",
  shortcut: null,
  autoCopy: true,
  saveAudioFiles: false,
  saveTranscriptionHistory: true,
  selectedMicrophone: "default"
});

export function createDefaultAppSettings(): AppSettings {
  return { ...defaultAppSettings };
}

export function normalizeAppSettings(settings: Partial<AppSettings> | null | undefined): AppSettings {
  const value = settings ?? {};

  return {
    provider: normalizeProvider(value.provider),
    defaultLanguage: normalizeStringSetting(value.defaultLanguage, defaultAppSettings.defaultLanguage),
    shortcut: normalizeOptionalStringSetting(value.shortcut),
    autoCopy: normalizeBooleanSetting(value.autoCopy, defaultAppSettings.autoCopy),
    saveAudioFiles: normalizeBooleanSetting(value.saveAudioFiles, defaultAppSettings.saveAudioFiles),
    saveTranscriptionHistory: normalizeBooleanSetting(
      value.saveTranscriptionHistory,
      defaultAppSettings.saveTranscriptionHistory
    ),
    selectedMicrophone: normalizeStringSetting(
      value.selectedMicrophone,
      defaultAppSettings.selectedMicrophone
    )
  };
}

export function normalizeStoredAppSettings(
  settings: Partial<Record<AppSettingsStoreKey, unknown>> | null | undefined
): AppSettings {
  const value = settings ?? {};

  return {
    provider: normalizeProvider(value.default_provider),
    defaultLanguage: normalizeStringSetting(value.default_language, defaultAppSettings.defaultLanguage),
    shortcut: normalizeOptionalStringSetting(value.shortcut),
    autoCopy: normalizeBooleanSetting(value.auto_copy, defaultAppSettings.autoCopy),
    saveAudioFiles: normalizeBooleanSetting(value.save_audio_files, defaultAppSettings.saveAudioFiles),
    saveTranscriptionHistory: normalizeBooleanSetting(
      value.save_transcription_history,
      defaultAppSettings.saveTranscriptionHistory
    ),
    selectedMicrophone: normalizeStringSetting(
      value.selected_microphone,
      defaultAppSettings.selectedMicrophone
    )
  };
}

export function toStoredAppSettings(settings: AppSettings): StoredAppSettings {
  const normalized = normalizeAppSettings(settings);

  return {
    default_provider: normalized.provider,
    default_language: normalized.defaultLanguage,
    shortcut: normalized.shortcut,
    auto_copy: normalized.autoCopy,
    save_audio_files: normalized.saveAudioFiles,
    save_transcription_history: normalized.saveTranscriptionHistory,
    selected_microphone: normalized.selectedMicrophone
  };
}

function normalizeProvider(value: unknown): ProviderId {
  return typeof value === "string" && providerIds.includes(value as ProviderId)
    ? (value as ProviderId)
    : defaultAppSettings.provider;
}

function normalizeStringSetting(value: unknown, fallback: string): string {
  if (typeof value !== "string") {
    return fallback;
  }

  const normalized = value.trim();

  return normalized.length > 0 ? normalized : fallback;
}

function normalizeOptionalStringSetting(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const normalized = value.trim();

  return normalized.length > 0 ? normalized : null;
}

function normalizeBooleanSetting(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}
