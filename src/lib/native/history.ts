import { invoke } from "@tauri-apps/api/core";

export interface HistoryTranscriptionSummary {
  id: string;
  text: string;
  provider: string;
  model: string | null;
  language: string | null;
  durationMs: number | null;
  copiedToClipboard: boolean;
  hasAudioFile: boolean;
  hasError: boolean;
  createdAt: string;
}

export interface HistoryTranscription {
  id: string;
  text: string;
  provider: string;
  model: string | null;
  language: string | null;
  durationMs: number | null;
  audioPath: string | null;
  audioDeleted: boolean;
  copiedToClipboard: boolean;
  error: string | null;
  createdAt: string;
}

export interface DeleteTranscriptionResult {
  deleted: boolean;
}

export interface ClearHistoryResult {
  deletedCount: number;
}

export async function getHistory(): Promise<HistoryTranscriptionSummary[]> {
  return invoke<HistoryTranscriptionSummary[]>("get_history");
}

export async function getTranscription(id: string): Promise<HistoryTranscription | null> {
  return invoke<HistoryTranscription | null>("get_transcription", { id });
}

export async function deleteTranscription(id: string): Promise<DeleteTranscriptionResult> {
  return invoke<DeleteTranscriptionResult>("delete_transcription", { id });
}

export async function clearHistory(): Promise<ClearHistoryResult> {
  return invoke<ClearHistoryResult>("clear_history");
}
