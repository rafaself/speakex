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
    blurb: "Saved transcripts locally"
  },
  {
    id: "settings",
    label: "Settings",
    blurb: "Provider and app preferences"
  }
];

export const recordingPlanSteps = [
  "Load the available microphones from Rust before starting capture.",
  "Poll explicit recorder status while capture is active so the UI can show elapsed, remaining, and max duration safely.",
  "Keep mock transcription separate while Release 0.9 adds an explicit Gemini path for completed local recordings."
];

const appStatusByPhase: Record<RecordingPhase, AppStatus> = {
  idle: {
    phase: "idle",
    phaseLabel: "Idle",
    headline: "Ready to capture a local recording.",
    detail:
      "Release 0.9 keeps recording explicit while adding a manual Gemini transcription path for completed local recordings.",
    transcriptTitle: "No recorded audio yet.",
    transcriptPreview:
      "Start a recording to create a temporary WAV file, then run either the separate mock path or the explicit Gemini path once audio is ready.",
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
      "Audio capture is active through Rust. Use Stop to keep the temporary WAV file or Cancel to discard it.",
    transcriptTitle: "Live capture in progress…",
    transcriptPreview:
      "The app is recording into a temporary local WAV file. No transcription will run automatically when capture ends, including when the 15-minute cap is reached.",
    inputLabel: "System default microphone",
    durationLabel: "Recording…",
    recordingTiming: null,
    recordedAudio: null
  },
  transcribing: {
    phase: "transcribing",
    phaseLabel: "Transcribing",
    headline: "Mock transcription is running.",
    detail:
      "The frontend is waiting on the explicit Rust mock transcription command. Recorded audio is not sent into this mock flow.",
    transcriptTitle: "Draft transcript incoming…",
    transcriptPreview:
      "Transcription remains explicit in Release 0.9 so mock and Gemini runs stay manual and separate from the recording lifecycle.",
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
      "The recording stopped successfully and the temporary WAV file is ready for an explicit mock or Gemini transcription request. No transcription ran automatically.",
    transcriptTitle: "Recorded audio metadata",
    transcriptPreview:
      "The capture finished successfully. Review the local audio details below, then choose either the mock path or Gemini manually.",
    inputLabel: "System default microphone",
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  },
  error: {
    phase: "error",
    phaseLabel: "Error",
    headline: "Recording workflow error.",
    detail:
      "The requested recorder action did not finish. The UI keeps the mock transcription path separate from recording errors.",
    transcriptTitle: "Native recording error",
    transcriptPreview:
      "The recorder command returned an error. Adjust the microphone choice or retry the action.",
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
    blurb: "Remote provider planned for the first MVP path.",
    note: "Store the API key in the OS keychain here; the recording screen can then invoke Gemini manually for a completed recording."
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
