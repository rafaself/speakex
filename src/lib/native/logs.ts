import { invoke } from "@tauri-apps/api/core";

export interface ErrorLogEntry {
  id: string;
  scope: string;
  source: string;
  summary: string;
  detail: string;
  createdAt: string;
}

export interface ClearErrorLogsResult {
  deletedCount: number;
}

export interface CreateErrorLogInput {
  scope: string;
  source: string;
  summary: string;
  detail: string;
}

export async function getErrorLogs(): Promise<ErrorLogEntry[]> {
  return invoke<ErrorLogEntry[]>("get_error_logs");
}

export async function clearErrorLogs(): Promise<ClearErrorLogsResult> {
  return invoke<ClearErrorLogsResult>("clear_error_logs");
}

export async function createErrorLog(entry: CreateErrorLogInput): Promise<void> {
  return invoke<void>("create_error_log", { request: entry });
}