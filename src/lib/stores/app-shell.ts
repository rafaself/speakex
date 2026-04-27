import { derived, writable } from "svelte/store";

import { createDefaultAppSettings } from "$lib/settings/schema";

import type {
  AppSection,
  AppStatus,
  NavigationSection,
  RecordingPhase,
  ProviderId,
  ProviderOption,
  SettingsDraft
} from "$lib/types/app-shell";

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
  "Choose an input source and capture audio from the desktop app.",
  "Review the generated transcript before sending it elsewhere.",
  "Keep a lightweight local history for quick reuse."
];

const appStatusByPhase: Record<RecordingPhase, AppStatus> = {
  idle: {
    phase: "idle",
    phaseLabel: "Idle",
    headline: "Record speech and keep the transcript close at hand.",
    detail:
      "This is a local-only mock. Use the phase controls to preview how the recording workspace will react before any real native capture exists.",
    transcriptTitle: "No transcript yet.",
    transcriptPreview:
      "Captured text, copy actions, and provider metadata will appear here after the native pipeline exists.",
    inputLabel: "Default microphone (planned)",
    durationLabel: "00:00"
  },
  recording: {
    phase: "recording",
    phaseLabel: "Recording",
    headline: "Mock capture is in progress.",
    detail:
      "No audio is being recorded. This state only demonstrates how the interface will look while a future Rust recorder is active.",
    transcriptTitle: "Listening for speech…",
    transcriptPreview:
      "A live waveform or timer is not implemented in Release 0.2. This placeholder simply marks the app as actively recording.",
    inputLabel: "Desk USB microphone (mock)",
    durationLabel: "00:18"
  },
  transcribing: {
    phase: "transcribing",
    phaseLabel: "Transcribing",
    headline: "Mock processing has started.",
    detail:
      "The app is pretending to hand recorded audio to Gemini, but no request, provider execution, or native work happens in this release.",
    transcriptTitle: "Draft transcript incoming…",
    transcriptPreview:
      "Frontend-only loading state. A later release will replace this with a real transcription pipeline and provider results.",
    inputLabel: "Desk USB microphone (mock)",
    durationLabel: "00:18"
  },
  completed: {
    phase: "completed",
    phaseLabel: "Completed",
    headline: "Mock transcript ready for review.",
    detail:
      "This sample output is hard-coded in the frontend so the shell can demonstrate the completed layout without saving, copying, or sending anything.",
    transcriptTitle: "Fake transcript preview",
    transcriptPreview:
      "Standup notes: finalize the shell, keep Gemini as the only provider, and leave recording plus persistence for later releases.",
    inputLabel: "Desk USB microphone (mock)",
    durationLabel: "00:18"
  },
  error: {
    phase: "error",
    phaseLabel: "Error",
    headline: "Mock failure state surfaced.",
    detail:
      "This error is intentionally fake and local-only. It exists so the UI can reserve space for retry guidance and visible failure messaging.",
    transcriptTitle: "Mock processing error",
    transcriptPreview:
      "The pretend transcription step could not finish. No audio was lost because no recording or provider call actually happened.",
    inputLabel: "Desk USB microphone (mock)",
    durationLabel: "00:18"
  }
};

const mockPhaseOrder: RecordingPhase[] = ["idle", "recording", "transcribing", "completed"];

const initialAppStatus = appStatusByPhase.idle;

function createAppStatusStore() {
  const { subscribe, set, update } = writable(initialAppStatus);

  return {
    subscribe,
    setPhase: (phase: RecordingPhase) => set(appStatusByPhase[phase]),
    advance: () =>
      update((status) => {
        const currentIndex = mockPhaseOrder.indexOf(status.phase);
        const nextPhase = mockPhaseOrder[(currentIndex + 1) % mockPhaseOrder.length] ?? "idle";

        return appStatusByPhase[nextPhase];
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
    note: "Draft only — no API key or request flow yet."
  }
];

export const languageOptions = [
  { value: "auto", label: "Auto-detect" },
  { value: "en-US", label: "English (US)" },
  { value: "pt-BR", label: "Português (Brasil)" }
];

export const microphoneOptions = [
  { value: "default", label: "System default microphone" },
  { value: "desk-usb", label: "Desk USB microphone" },
  { value: "headset", label: "Headset microphone" }
];

export const mockRecordingPhases = mockPhaseOrder.map((phase) => appStatusByPhase[phase]).concat(appStatusByPhase.error);

export const activeSection = writable<AppSection>("recording");
export const appStatus = createAppStatusStore();
export const settingsDraft = createSettingsDraftStore();

export const providerSelection = {
  subscribe(run: (value: ProviderId) => void) {
    return derived(settingsDraft, ($settingsDraft) => $settingsDraft.provider).subscribe(run);
  },
  set(provider: ProviderId) {
    settingsDraft.patch({ provider });
  }
};
