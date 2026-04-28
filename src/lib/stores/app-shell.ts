import { derived, writable } from "svelte/store";

import { createDefaultAppSettings } from "$lib/settings/schema";

import type {
  AppSection,
  AppStatus,
  NavigationSection,
  RecordedAudioMetadata,
  RecordingInputOption,
  RecordingPhase,
  ProviderId,
  ProviderOption,
  SettingsDraft
} from "$lib/types/app-shell";

interface RecordingInputDeviceLike {
  name: string;
  isDefault: boolean;
}

export const navigationSections: NavigationSection[] = [
  {
    id: "recording",
    label: "Record",
    blurb: "Primary capture workflow"
  },
  {
    id: "history",
    label: "History",
    blurb: "Saved transcripts and outcomes"
  },
  {
    id: "settings",
    label: "Settings",
    blurb: "Provider and app preferences"
  }
];

export const recordingPlanSteps = [
  "Choose from the available microphones before you start recording.",
  "While recording is active, show elapsed time, remaining time, and the maximum duration clearly.",
  "Stopping keeps the recording ready for manual Gemini transcription. If a manual run fails, Retry stays available while the recorded audio file still exists, and hidden-window failure notifications stay generic."
];

const appStatusByPhase: Record<RecordingPhase, AppStatus> = {
  idle: {
    phase: "idle",
    phaseLabel: "Idle",
    headline: "Ready to capture a local recording.",
    detail:
      "Recording stays separate from transcription. After you stop, you can run Gemini manually. Manual results can copy to the clipboard, save to history, clean up audio by default, and send desktop notifications only while SpeakEx is hidden.",
    transcriptTitle: "No recorded audio yet.",
    transcriptPreview:
      "Start a recording to create a temporary audio file, then run Gemini when you're ready. If a manual run fails before cleanup, Retry stays available while that audio file still exists.",
    inputLabel: "System default microphone",
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  },
  recording: {
    phase: "recording",
    phaseLabel: "Recording",
    headline: "Recording is in progress.",
    detail:
      "Audio capture is active. Use Stop to keep the current audio file or Cancel to discard it.",
    transcriptTitle: "Live capture in progress…",
    transcriptPreview:
      "SpeakEx is recording to a temporary local audio file. No transcription runs automatically when capture ends, including at the 15-minute limit.",
    inputLabel: "System default microphone",
    durationLabel: "Recording…",
    recordingTiming: null,
    recordedAudio: null
  },
  transcribing: {
    phase: "transcribing",
    phaseLabel: "Transcribing",
    headline: "Manual transcription is running.",
    detail:
      "SpeakEx is transcribing the current recorded audio with Gemini.",
    transcriptTitle: "Draft transcript incoming…",
    transcriptPreview:
      "Gemini transcription stays separate from recording, and Retry only applies to the current recorded audio file while it is still available.",
    inputLabel: "System default microphone",
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  },
  completed: {
    phase: "completed",
    phaseLabel: "Completed",
    headline: "Recorded audio is ready.",
    detail:
      "Recording finished successfully and the audio file is ready for manual Gemini transcription. No transcription ran automatically, and Retry only applies while this file still exists.",
    transcriptTitle: "Recorded audio metadata",
    transcriptPreview: "Review the audio details below, then run the Gemini transcription flow.",
    inputLabel: "System default microphone",
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  },
  error: {
    phase: "error",
    phaseLabel: "Error",
    headline: "Recording workflow error.",
    detail: "Recording ran into a problem. Check your microphone selection and try again.",
    transcriptTitle: "Native recording error",
    transcriptPreview:
      "SpeakEx could not finish the recording action. Check your microphone selection and try again.",
    inputLabel: "System default microphone",
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  }
};

const initialAppStatus = appStatusByPhase.idle;

export const defaultRecordingInputOption: RecordingInputOption = {
  value: "default",
  label: "System default microphone",
  isDefault: true
};

export function createRecordingInputOptions(
  devices: RecordingInputDeviceLike[],
  selectedMicrophone: string
): RecordingInputOption[] {
  const defaultDevice = devices.find((device) => device.isDefault);
  const options: RecordingInputOption[] = [
    {
      ...defaultRecordingInputOption,
      label: defaultDevice
        ? `System default microphone — ${defaultDevice.name}`
        : defaultRecordingInputOption.label
    }
  ];
  const knownValues = new Set<string>([defaultRecordingInputOption.value]);

  for (const device of devices) {
    if (knownValues.has(device.name)) {
      continue;
    }

    options.push({
      value: device.name,
      label: device.isDefault ? `${device.name} (default device)` : device.name,
      isDefault: device.isDefault
    });
    knownValues.add(device.name);
  }

  if (selectedMicrophone !== "default" && !knownValues.has(selectedMicrophone)) {
    options.push({
      value: selectedMicrophone,
      label: `${selectedMicrophone} (unavailable)`,
      isDefault: false,
      unavailable: true
    });
  }

  return options;
}

export function getAppStatusForPhase(
  phase: RecordingPhase,
  overrides: Partial<Omit<AppStatus, "phase">> = {}
): AppStatus {
  return {
    ...appStatusByPhase[phase],
    ...overrides,
    phase,
    recordingTiming: overrides.recordingTiming ?? appStatusByPhase[phase].recordingTiming,
    recordedAudio: overrides.recordedAudio ?? appStatusByPhase[phase].recordedAudio
  };
}

function createAppStatusStore() {
  const { subscribe, set } = writable(initialAppStatus);

  return {
    subscribe,
    setStatus: (status: AppStatus) => set(status),
    setPhase: (phase: RecordingPhase) => set(appStatusByPhase[phase]),
    setRecordedAudio: (recordedAudio: RecordedAudioMetadata | null) =>
      set({
        ...initialAppStatus,
        recordedAudio,
        recordingTiming: null
      }),
    reset: () => set(initialAppStatus)
  };
}

const initialSettingsDraft = (): SettingsDraft => createDefaultAppSettings();

export type DraftToggleKey = "autoCopy" | "saveAudioFiles" | "saveTranscriptionHistory";

function createSettingsDraftStore() {
  const { subscribe, set, update } = writable(initialSettingsDraft());

  return {
    subscribe,
    patch: (value: Partial<SettingsDraft>) => update((draft) => ({ ...draft, ...value })),
    reset: () => set(initialSettingsDraft()),
    toggle: (key: DraftToggleKey) =>
      update((draft) => ({
        ...draft,
        [key]: !draft[key]
      }))
  };
}

export const providerOptions: ProviderOption[] = [
  {
    id: "gemini",
    label: "Gemini",
    blurb: "Remote transcription provider for the current MVP.",
    note: "Save the API key in the OS keychain to enable manual Gemini transcription, optional clipboard copy, history saving, and automatic audio cleanup."
  }
];

export const languageOptions = [
  { value: "auto", label: "Auto-detect" },
  { value: "en-US", label: "English (US)" },
  { value: "pt-BR", label: "Português (Brasil)" }
];

export const activeSection = writable<AppSection>("recording");
export const appStatus = createAppStatusStore();
export const recordingInputOptions = writable<RecordingInputOption[]>([defaultRecordingInputOption]);
export const settingsDraft = createSettingsDraftStore();

export const providerSelection = {
  subscribe(run: (value: ProviderId) => void) {
    return derived(settingsDraft, ($settingsDraft) => $settingsDraft.provider).subscribe(run);
  },
  set(provider: ProviderId) {
    settingsDraft.patch({ provider });
  }
};
