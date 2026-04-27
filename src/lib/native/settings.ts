import { LazyStore } from "@tauri-apps/plugin-store";

import {
  appSettingsStoreKeyList,
  appSettingsStorePath,
  normalizeStoredAppSettings,
  toStoredAppSettings,
  type AppSettings,
  type AppSettingsStoreKey,
  type StoredAppSettings
} from "$lib/settings/schema";

const settingsStore = new LazyStore(appSettingsStorePath, {
  defaults: {},
  autoSave: false
});

export async function loadAppSettings(): Promise<AppSettings> {
  const storedSettings = await readStoredAppSettings();
  const settings = normalizeStoredAppSettings(storedSettings);
  const normalizedStoredSettings = toStoredAppSettings(settings);

  if (storedSettingsNeedInitialization(storedSettings, normalizedStoredSettings)) {
    await persistStoredAppSettings(normalizedStoredSettings);
  }

  return settings;
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  const normalizedStoredSettings = toStoredAppSettings(settings);

  await persistStoredAppSettings(normalizedStoredSettings);

  return normalizeStoredAppSettings(normalizedStoredSettings);
}

async function readStoredAppSettings(): Promise<Partial<Record<AppSettingsStoreKey, unknown>>> {
  const entries = await Promise.all(
    appSettingsStoreKeyList.map(async (key) => [key, await settingsStore.get(key)] as const)
  );

  return Object.fromEntries(entries);
}

async function persistStoredAppSettings(settings: StoredAppSettings): Promise<void> {
  await Promise.all(
    Object.entries(settings).map(([key, value]) => settingsStore.set(key, value))
  );

  await settingsStore.save();
}

function storedSettingsNeedInitialization(
  storedSettings: Partial<Record<AppSettingsStoreKey, unknown>>,
  normalizedStoredSettings: StoredAppSettings
): boolean {
  return Object.entries(normalizedStoredSettings).some(
    ([key, value]) => storedSettings[key as AppSettingsStoreKey] !== value
  );
}
