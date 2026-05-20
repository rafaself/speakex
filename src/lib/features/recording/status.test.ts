import { describe, expect, it } from "vitest";

import {
  buildCompletedManualStatus,
  buildCompletedRecordingStatus,
  buildFallbackRecordingStatus,
  buildMissingManualRecordingStatus,
  buildRetainedRecordedAudio,
  collectOutcomeWarnings
} from "./status";

const formatDuration = (durationMs: number | null) => (durationMs === null ? "—" : `${durationMs}ms`);
const formatFileName = (path: string) => path.split("/").pop() ?? path;

describe("recording status helpers", () => {
  it("builds a fallback active recording status snapshot", () => {
    expect(
      buildFallbackRecordingStatus(
        {
          id: "session-1",
          inputDeviceName: "USB Mic"
        },
        90_000
      )
    ).toEqual({
      phase: "recording",
      activeSessionId: "session-1",
      inputDeviceName: "USB Mic",
      elapsedMs: 0,
      remainingMs: 90_000,
      maxDurationMs: 90_000,
      limitReached: false,
      lastCompletedSessionId: null
    });
  });

  it("builds the completed recording app status with local audio metadata", () => {
    const status = buildCompletedRecordingStatus({
      stoppedRecording: {
        sessionId: "session-1",
        audioInput: {
          path: "/tmp/recordings/session-1.wav",
          mimeType: "audio/wav",
          durationMs: 42_000
        },
        inputDeviceName: "USB Mic",
        sampleRateHz: 48_000,
        channels: 2,
        fileSizeBytes: 128_000,
        limitReached: false
      },
      maxDurationMs: 900_000,
      formatDuration,
      formatFileName
    });

    expect(status).toMatchObject({
      phase: "completed",
      headline: "Recorded audio is ready.",
      transcriptPreview: "session-1.wav is available locally and ready for Gemini transcription.",
      inputLabel: "USB Mic"
    });
    expect(status.recordedAudio).toMatchObject({
      path: "/tmp/recordings/session-1.wav",
      durationMs: 42_000,
      maxDurationMs: 900_000
    });
  });

  it("collects only non-empty outcome warnings", () => {
    expect(
      collectOutcomeWarnings({
        transcript: {
          text: "Hello",
          provider: "Gemini",
          model: "1.5-pro",
          language: "en-US",
          durationMs: 5_000
        },
        historyId: "history-1",
        historySaved: true,
        historyError: "History warning",
        copiedToClipboard: true,
        clipboardError: "",
        pastedToActiveInput: false,
        pasteError: null,
        audioDeleted: false,
        audioDeleteError: null,
        retainedAudioPath: "/tmp/audio.wav"
      })
    ).toEqual(["History warning"]);
  });

  it("builds a completed manual transcription status with warnings", () => {
    const recordedAudio = {
      sessionId: "session-1",
      path: "/tmp/session-1.wav",
      mimeType: "audio/wav",
      durationMs: 42_000,
      inputDeviceName: "USB Mic",
      sampleRateHz: 48_000,
      channels: 2,
      fileSizeBytes: 128_000,
      limitReached: false,
      maxDurationMs: 900_000
    };

    const status = buildCompletedManualStatus({
      result: {
        transcript: {
          text: "Transcript ready",
          provider: "Gemini",
          model: "1.5-pro",
          language: "en-US",
          durationMs: 42_000
        },
        historyId: "history-1",
        historySaved: true,
        historyError: "History warning",
        copiedToClipboard: false,
        clipboardError: "Clipboard warning",
        pastedToActiveInput: false,
        pasteError: null,
        audioDeleted: false,
        audioDeleteError: null,
        retainedAudioPath: "/tmp/retained.wav"
      },
      recordedAudio,
      retainedRecordedAudio: {
        ...recordedAudio,
        path: "/tmp/retained.wav"
      },
      settings: {
        autoCopy: true,
        saveAudioFiles: true,
        saveTranscriptionHistory: true
      },
      hiddenManualNotificationCompletedMessage: "Notification note.",
      formatDuration
    });

    expect(status).toMatchObject({
      phase: "completed",
      headline: "Transcript ready with warnings.",
      transcriptTitle: "Transcript saved locally with warnings",
      transcriptPreview: "Transcript ready"
    });
    expect(status.detail).toContain("Saved to local history with warnings");
    expect(status.detail).toContain("Clipboard copy failed");
    expect(status.detail).toContain("retained");
    expect(status.recordedAudio?.path).toBe("/tmp/retained.wav");
  });

  it("clears the retained audio metadata when cleanup succeeds", () => {
    expect(
      buildRetainedRecordedAudio(
        {
          sessionId: "session-1",
          path: "/tmp/session-1.wav",
          mimeType: "audio/wav",
          durationMs: 42_000,
          inputDeviceName: "USB Mic",
          sampleRateHz: 48_000,
          channels: 2,
          fileSizeBytes: 128_000,
          limitReached: false,
          maxDurationMs: 900_000
        },
        {
          transcript: {
            text: "Transcript ready",
            provider: "Gemini",
            model: "1.5-pro",
            language: "en-US",
            durationMs: 42_000
          },
          historyId: null,
          historySaved: false,
          historyError: null,
          copiedToClipboard: false,
          clipboardError: null,
          pastedToActiveInput: false,
          pasteError: null,
          audioDeleted: true,
          audioDeleteError: null,
          retainedAudioPath: null
        }
      )
    ).toBeNull();
  });

  it("builds an explicit error status when manual transcription audio is missing", () => {
    expect(
      buildMissingManualRecordingStatus({
        recordedAudio: {
          sessionId: "session-1",
          path: "/tmp/session-1.wav",
          mimeType: "audio/wav",
          durationMs: 42_000,
          inputDeviceName: "USB Mic",
          sampleRateHz: 48_000,
          channels: 2,
          fileSizeBytes: 128_000,
          limitReached: false,
          maxDurationMs: 900_000
        },
        detail: "The file is gone.",
        transcriptPreview: "Retry is not available.",
        formatDuration
      })
    ).toMatchObject({
      phase: "error",
      headline: "Manual transcription needs a new recording.",
      detail: "The file is gone.",
      transcriptPreview: "Retry is not available."
    });
  });
});
