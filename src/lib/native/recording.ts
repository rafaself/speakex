import { invoke } from "@tauri-apps/api/core";

export interface RecordingInputDevice {
  name: string;
  label: string;
  isDefault: boolean;
}

export interface ActiveRecordingSession {
  id: string;
  inputDeviceName: string;
}

export interface RecordingStatus {
  phase: "idle" | "starting" | "recording" | "stopping" | "cancelling";
  activeSessionId: string | null;
  inputDeviceName: string | null;
  elapsedMs: number | null;
  remainingMs: number | null;
  maxDurationMs: number;
  limitReached: boolean;
  lastCompletedSessionId: string | null;
}

export interface RecordedAudioInput {
  path: string;
  mimeType: string;
  durationMs: number | null;
}

export interface StoppedRecording {
  sessionId: string;
  audioInput: RecordedAudioInput;
  inputDeviceName: string;
  sampleRateHz: number;
  channels: number;
  fileSizeBytes: number;
  limitReached: boolean;
}

export interface CancelledRecording {
  sessionId: string;
  deletedAudioPath: string | null;
}

export async function listRecordingInputDevices(): Promise<RecordingInputDevice[]> {
  return invoke<RecordingInputDevice[]>("list_recording_input_devices");
}

export async function startRecording(deviceName: string | null): Promise<ActiveRecordingSession> {
  return invoke<ActiveRecordingSession>("start_recording", { deviceName });
}

export async function getRecordingStatus(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("get_recording_status");
}

export async function stopRecording(): Promise<StoppedRecording> {
  return invoke<StoppedRecording>("stop_recording");
}

export async function cancelRecording(): Promise<CancelledRecording> {
  return invoke<CancelledRecording>("cancel_recording");
}
