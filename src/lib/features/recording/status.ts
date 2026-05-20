import { getAppStatusForPhase } from "$lib/stores/app-shell";

import type { ActiveRecordingSession, RecordingStatus, StoppedRecording } from "$lib/native/recording";
import type { RunCompletedRecordingTranscriptionResult } from "$lib/native/transcription";
import type { RecordedAudioMetadata, RecordingTiming } from "$lib/types/app-shell";

interface FormattingDeps {
  formatDuration: (durationMs: number | null) => string;
  formatFileName?: (path: string) => string;
}

interface ManualOutcomeSettingsSnapshot {
  autoCopy: boolean;
  saveAudioFiles: boolean;
  saveTranscriptionHistory: boolean;
}

export function buildFallbackRecordingStatus(
  session: ActiveRecordingSession,
  maxDurationMs: number
): RecordingStatus {
  return {
    phase: "recording",
    activeSessionId: session.id,
    inputDeviceName: session.inputDeviceName,
    elapsedMs: 0,
    remainingMs: maxDurationMs,
    maxDurationMs,
    limitReached: false,
    lastCompletedSessionId: null
  };
}

export function buildRecordingTimingFromStatus(status: RecordingStatus): RecordingTiming {
  return {
    elapsedMs: status.elapsedMs,
    remainingMs: status.remainingMs,
    maxDurationMs: status.maxDurationMs,
    limitReached: status.limitReached
  };
}

export function buildRecordingTimingFromRecordedAudio(
  recordedAudio: RecordedAudioMetadata
): RecordingTiming {
  return {
    elapsedMs: recordedAudio.durationMs,
    remainingMs:
      recordedAudio.durationMs === null || recordedAudio.maxDurationMs === null
        ? null
        : Math.max(recordedAudio.maxDurationMs - recordedAudio.durationMs, 0),
    maxDurationMs: recordedAudio.maxDurationMs,
    limitReached: recordedAudio.limitReached
  };
}

export function buildDurationSummaryLabel(
  recordingTiming: RecordingTiming | null,
  { formatDuration }: FormattingDeps
): string {
  if (!recordingTiming) {
    return "—";
  }

  return `${formatDuration(recordingTiming.elapsedMs)} elapsed · ${formatDuration(recordingTiming.remainingMs)} remaining`;
}

export function buildActiveRecordingStatus(args: {
  status: RecordingStatus;
  selectedMicrophoneLabel: string;
  formatDuration: FormattingDeps["formatDuration"];
}) {
  const recordingTiming = buildRecordingTimingFromStatus(args.status);

  return getAppStatusForPhase("recording", {
    headline:
      args.status.phase === "starting"
        ? "Recording is starting."
        : args.status.phase === "stopping"
          ? "Recording is stopping."
          : args.status.phase === "cancelling"
            ? "Recording is cancelling."
            : "Recording is in progress.",
    detail: `Audio capture is active and will stop automatically at ${args.formatDuration(args.status.maxDurationMs)}. Use the check button to stop and send the audio to Gemini, or X to discard it sooner.`,
    transcriptTitle: "Live capture in progress…",
    transcriptPreview: `Recording session ${args.status.activeSessionId ?? "current"} is saving a temporary audio file locally. Nothing is sent until you confirm it with the check button, including before the duration limit is reached.`,
    inputLabel: args.status.inputDeviceName ?? args.selectedMicrophoneLabel,
    durationLabel: buildDurationSummaryLabel(recordingTiming, args),
    recordingTiming,
    recordedAudio: null
  });
}

export function mapStoppedRecording(
  stoppedRecording: StoppedRecording,
  maxDurationMs: number
): RecordedAudioMetadata {
  return {
    sessionId: stoppedRecording.sessionId,
    path: stoppedRecording.audioInput.path,
    mimeType: stoppedRecording.audioInput.mimeType,
    durationMs: stoppedRecording.audioInput.durationMs,
    inputDeviceName: stoppedRecording.inputDeviceName,
    sampleRateHz: stoppedRecording.sampleRateHz,
    channels: stoppedRecording.channels,
    fileSizeBytes: stoppedRecording.fileSizeBytes,
    limitReached: stoppedRecording.limitReached,
    maxDurationMs
  };
}

export function buildCompletedRecordingStatus(args: {
  stoppedRecording: StoppedRecording;
  maxDurationMs: number;
  formatDuration: FormattingDeps["formatDuration"];
  formatFileName: NonNullable<FormattingDeps["formatFileName"]>;
}) {
  const recordedAudio = mapStoppedRecording(args.stoppedRecording, args.maxDurationMs);
  const recordingTiming = buildRecordingTimingFromRecordedAudio(recordedAudio);

  return getAppStatusForPhase("completed", {
    headline: args.stoppedRecording.limitReached
      ? `Recording stopped at the ${args.formatDuration(recordingTiming.maxDurationMs)} limit.`
      : "Recorded audio is ready.",
    detail: args.stoppedRecording.limitReached
      ? `Capture stopped automatically because the maximum recording duration of ${args.formatDuration(recordingTiming.maxDurationMs)} was reached. The audio file was kept locally, and no transcription ran automatically.`
      : "Recording stopped successfully and kept the audio file locally. Run Gemini transcription when you are ready.",
    transcriptTitle: "Recorded audio metadata",
    transcriptPreview: args.stoppedRecording.limitReached
      ? `${args.formatFileName(recordedAudio.path)} was captured locally after the automatic safety stop. Run Gemini transcription when you are ready.`
      : `${args.formatFileName(recordedAudio.path)} is available locally and ready for Gemini transcription.`,
    inputLabel: args.stoppedRecording.inputDeviceName,
    durationLabel: buildDurationSummaryLabel(recordingTiming, args),
    recordingTiming,
    recordedAudio
  });
}

export function collectOutcomeWarnings(
  result: RunCompletedRecordingTranscriptionResult
): string[] {
  return [result.historyError, result.clipboardError, result.pasteError, result.audioDeleteError].filter(
    (value): value is string => typeof value === "string" && value.trim() !== ""
  );
}

export function describeManualHistoryOutcome(
  result: RunCompletedRecordingTranscriptionResult,
  saveTranscriptionHistory: boolean
): string {
  if (result.historySaved) {
    return collectOutcomeWarnings(result).length === 0
      ? "Saved to local history."
      : "Saved to local history with warnings recorded on the entry.";
  }

  if (result.historyError) {
    return "Could not save to local history, so this transcript stays in the current preview.";
  }

  return saveTranscriptionHistory
    ? "Local history was not updated."
    : "Local history was skipped because Save transcription history is turned off.";
}

export function describeManualClipboardOutcome(
  result: RunCompletedRecordingTranscriptionResult,
  autoCopy: boolean
): string {
  if (result.copiedToClipboard) {
    return "Copied to the system clipboard.";
  }

  if (result.clipboardError) {
    return "Clipboard copy failed, so the transcript stayed in SpeakEx only.";
  }

  return autoCopy
    ? "Clipboard copy was requested but did not finish."
    : "Clipboard copy was skipped because Auto-copy transcript is turned off.";
}

export function describeManualAudioOutcome(
  result: RunCompletedRecordingTranscriptionResult,
  saveAudioFiles: boolean
): string {
  if (result.audioDeleted && !result.audioDeleteError) {
    return "The local audio file was deleted after transcription.";
  }

  if (result.audioDeleteError) {
    return "Audio cleanup failed, so the local audio file was retained.";
  }

  if (result.retainedAudioPath) {
    return saveAudioFiles
      ? "The local audio file was retained because Save audio files is turned on."
      : "The local audio file was retained locally.";
  }

  return "The local audio file outcome is unavailable.";
}

export function buildCompletedManualStatus(args: {
  result: RunCompletedRecordingTranscriptionResult;
  recordedAudio: RecordedAudioMetadata;
  retainedRecordedAudio: RecordedAudioMetadata | null;
  settings: ManualOutcomeSettingsSnapshot;
  hiddenManualNotificationCompletedMessage: string;
  formatDuration: FormattingDeps["formatDuration"];
}) {
  const outcomeSummary = [
    describeManualHistoryOutcome(args.result, args.settings.saveTranscriptionHistory),
    describeManualClipboardOutcome(args.result, args.settings.autoCopy),
    describeManualAudioOutcome(args.result, args.settings.saveAudioFiles)
  ].join(" ");
  const warnings = collectOutcomeWarnings(args.result);

  return getAppStatusForPhase("completed", {
    headline: warnings.length === 0 ? "Transcript ready." : "Transcript ready with warnings.",
    detail: `${outcomeSummary} ${args.hiddenManualNotificationCompletedMessage}`,
    transcriptTitle: args.result.historySaved
      ? warnings.length === 0
        ? "Transcript saved locally"
        : "Transcript saved locally with warnings"
      : args.result.historyError
        ? "Transcript ready in preview only"
        : "Transcript ready in preview",
    transcriptPreview: args.result.transcript.text,
    inputLabel: args.recordedAudio.inputDeviceName,
    durationLabel: args.formatDuration(args.result.transcript.durationMs ?? args.recordedAudio.durationMs),
    recordingTiming: buildRecordingTimingFromRecordedAudio(args.recordedAudio),
    recordedAudio: args.retainedRecordedAudio
  });
}

export function buildRetainedRecordedAudio(
  recordedAudio: RecordedAudioMetadata,
  result: RunCompletedRecordingTranscriptionResult
): RecordedAudioMetadata | null {
  if (result.audioDeleted && !result.audioDeleteError) {
    return null;
  }

  return {
    ...recordedAudio,
    path: result.retainedAudioPath ?? recordedAudio.path
  };
}

export function buildMissingManualRecordingStatus(args: {
  recordedAudio: RecordedAudioMetadata;
  detail: string;
  transcriptPreview: string;
  formatDuration: FormattingDeps["formatDuration"];
}) {
  return getAppStatusForPhase("error", {
    headline: "Manual transcription needs a new recording.",
    inputLabel: args.recordedAudio.inputDeviceName,
    detail: args.detail,
    transcriptPreview: args.transcriptPreview,
    durationLabel: args.formatDuration(args.recordedAudio.durationMs),
    recordingTiming: buildRecordingTimingFromRecordedAudio(args.recordedAudio),
    recordedAudio: null
  });
}

export function describeStoredClipboardOutcome(
  copiedToClipboard: boolean,
  storedIssue: string
): string {
  if (copiedToClipboard) {
    return "Copied to clipboard";
  }

  return storedIssue === ""
    ? "Not copied to clipboard"
    : "Not copied to clipboard (see warnings)";
}

export function describeStoredAudioOutcome(
  audioDeleted: boolean,
  audioPath: string | null
): string {
  if (audioDeleted) {
    return "Deleted after transcription";
  }

  return audioPath ? "Retained locally" : "No retained audio";
}
