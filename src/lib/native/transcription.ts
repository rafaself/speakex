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

export interface TranscriptionOptions {
  language?: string | null;
  prompt?: string | null;
  model?: string | null;
}

export interface RunMockTranscriptionResult {
  transcript: Transcript;
  savedToHistory: boolean;
}

export async function runMockTranscription(): Promise<RunMockTranscriptionResult> {
  return invoke<RunMockTranscriptionResult>("run_mock_transcription");
}

export async function runGeminiTranscription(
  audioInput: TranscriptionAudioInput,
  options: TranscriptionOptions = {}
): Promise<Transcript> {
  return invoke<Transcript>("run_gemini_transcription", {
    request: {
      audioInput,
      options: {
        language: options.language ?? null,
        prompt: options.prompt ?? null,
        model: options.model ?? null
      }
    }
  });
}
