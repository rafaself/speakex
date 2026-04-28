import { invoke } from "@tauri-apps/api/core";

export interface Transcript {
  text: string;
  provider: string;
  model: string | null;
  language: string | null;
  durationMs: number | null;
}

export interface TranscriptionAudioInput {
  path: string;
  mimeType: string;
  durationMs: number | null;
}

export interface RunMockTranscriptionResult {
  transcript: Transcript;
  savedToHistory: boolean;
}

export interface RunCompletedRecordingTranscriptionResult {
  transcript: Transcript;
  historyId: string | null;
  historySaved: boolean;
  historyError: string | null;
  copiedToClipboard: boolean;
  clipboardError: string | null;
  audioDeleted: boolean;
  audioDeleteError: string | null;
  retainedAudioPath: string | null;
}

export async function runMockTranscription(): Promise<RunMockTranscriptionResult> {
  return invoke<RunMockTranscriptionResult>("run_mock_transcription");
}

export async function runCompletedRecordingTranscription(
  audioInput: TranscriptionAudioInput
): Promise<RunCompletedRecordingTranscriptionResult> {
  return invoke<RunCompletedRecordingTranscriptionResult>("run_completed_recording_transcription", {
    request: {
      audioInput
    }
  });
}

export async function hasCompletedRecordingAudio(
  audioInput: TranscriptionAudioInput
): Promise<boolean> {
  return invoke<boolean>("has_completed_recording_audio", {
    request: {
      audioInput
    }
  });
}
