import { invoke } from "@tauri-apps/api/core";

export interface MockTranscript {
  text: string;
  provider: string;
  model: string | null;
  language: string | null;
  durationMs: number | null;
}

export interface RunMockTranscriptionResult {
  transcript: MockTranscript;
  savedToHistory: boolean;
}

export async function runMockTranscription(): Promise<RunMockTranscriptionResult> {
  return invoke<RunMockTranscriptionResult>("run_mock_transcription");
}
