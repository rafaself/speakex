import { invoke } from "@tauri-apps/api/core";

export async function saveGeminiApiKey(apiKey: string): Promise<void> {
  await invoke("save_gemini_api_key", { apiKey });
}

export async function hasGeminiApiKey(): Promise<boolean> {
  return invoke<boolean>("has_gemini_api_key");
}

export async function clearGeminiApiKey(): Promise<boolean> {
  return invoke<boolean>("clear_gemini_api_key");
}
