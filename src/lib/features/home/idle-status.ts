import { getAppStatusForPhase } from "$lib/stores/app-shell";
import type { AppStatus } from "$lib/types/app-shell";

type RecordingDevicesState = "loading" | "ready" | "error";

interface IdleStatusOptions {
  recordingDevicesState: RecordingDevicesState;
  recordingDevicesError: string;
  selectedMicrophoneUnavailable: boolean;
  selectedMicrophoneLabel: string;
  detail?: string;
}

export function buildIdleDetail(options: IdleStatusOptions): string {
  if (options.recordingDevicesState === "loading") {
    return "Loading available microphones before recording becomes available.";
  }

  if (options.recordingDevicesState === "error") {
    return `Unable to load recording inputs: ${options.recordingDevicesError}`;
  }

  if (options.selectedMicrophoneUnavailable) {
    return "The saved microphone is not currently available. Pick one of the loaded inputs before starting a recording.";
  }

  return `Ready to record from ${options.selectedMicrophoneLabel}. Stop keeps the audio file so you can run Gemini when you are ready.`;
}

export function buildIdleAppStatus(options: IdleStatusOptions): AppStatus {
  return getAppStatusForPhase("idle", {
    detail: options.detail ?? buildIdleDetail(options),
    transcriptPreview:
      options.recordingDevicesState === "error"
        ? "Microphone loading failed. Retry device loading while recording is unavailable."
        : options.selectedMicrophoneUnavailable
          ? "Choose an available microphone before starting a recording. Your saved selection stays in place until you update it."
          : "Start a recording to capture a temporary audio file locally, then run Gemini when you are ready.",
    inputLabel: options.selectedMicrophoneLabel,
    durationLabel: "—",
    recordingTiming: null,
    recordedAudio: null
  });
}