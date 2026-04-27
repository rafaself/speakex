import { invoke } from "@tauri-apps/api/core";

export interface RecordingInputDevice {
  name: string;
  isDefault: boolean;
}

export interface ActiveRecordingSession {
  id: string;
  inputDeviceName: string;
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

export async function stopRecording(): Promise<StoppedRecording> {
  return invoke<StoppedRecording>("stop_recording");
}

export async function cancelRecording(): Promise<CancelledRecording> {
  return invoke<CancelledRecording>("cancel_recording");
}
