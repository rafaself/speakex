import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";

import { createDefaultAppSettings } from "$lib/settings/schema";
import {
  activeSection,
  appStatus,
  defaultRecordingInputOption,
  recordingInputOptions,
  settingsDraft
} from "$lib/stores/app-shell";
import Page from "../routes/+page.svelte";

const pingMock = vi.hoisted(() => vi.fn());
const historyMocks = vi.hoisted(() => ({
  clearHistory: vi.fn(),
  deleteTranscription: vi.fn(),
  getHistory: vi.fn(),
  getTranscription: vi.fn()
}));
const recordingMocks = vi.hoisted(() => ({
  cancelRecording: vi.fn(),
  getRecordingStatus: vi.fn(),
  listRecordingInputDevices: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn()
}));
const transcriptionMocks = vi.hoisted(() => ({
  hasCompletedRecordingAudio: vi.fn(),
  runCompletedRecordingTranscription: vi.fn()
}));
const shortcutMocks = vi.hoisted(() => ({
  applyRecordingShortcut: vi.fn(),
  getRecordingShortcutStatus: vi.fn()
}));
const settingsMocks = vi.hoisted(() => ({
  loadAppSettings: vi.fn(),
  saveAppSettings: vi.fn()
}));
const secretStoreMocks = vi.hoisted(() => ({
  clearGeminiApiKey: vi.fn(),
  hasGeminiApiKey: vi.fn(),
  saveGeminiApiKey: vi.fn()
}));

vi.mock("$lib/native/ping", () => ({
  ping: pingMock
}));

vi.mock("$lib/native/history", () => ({
  clearHistory: historyMocks.clearHistory,
  deleteTranscription: historyMocks.deleteTranscription,
  getHistory: historyMocks.getHistory,
  getTranscription: historyMocks.getTranscription
}));

vi.mock("$lib/native/recording", () => ({
  cancelRecording: recordingMocks.cancelRecording,
  getRecordingStatus: recordingMocks.getRecordingStatus,
  listRecordingInputDevices: recordingMocks.listRecordingInputDevices,
  startRecording: recordingMocks.startRecording,
  stopRecording: recordingMocks.stopRecording
}));

vi.mock("$lib/native/transcription", () => ({
  hasCompletedRecordingAudio: transcriptionMocks.hasCompletedRecordingAudio,
  runCompletedRecordingTranscription: transcriptionMocks.runCompletedRecordingTranscription
}));

vi.mock("$lib/native/shortcut", () => ({
  applyRecordingShortcut: shortcutMocks.applyRecordingShortcut,
  getRecordingShortcutStatus: shortcutMocks.getRecordingShortcutStatus
}));

vi.mock("$lib/native/settings", () => ({
  loadAppSettings: settingsMocks.loadAppSettings,
  saveAppSettings: settingsMocks.saveAppSettings
}));

vi.mock("$lib/native/secret-store", () => ({
  clearGeminiApiKey: secretStoreMocks.clearGeminiApiKey,
  hasGeminiApiKey: secretStoreMocks.hasGeminiApiKey,
  saveGeminiApiKey: secretStoreMocks.saveGeminiApiKey
}));

function resetAppStores() {
  activeSection.set("recording");
  appStatus.reset();
  recordingInputOptions.set([defaultRecordingInputOption]);
  settingsDraft.reset();
}

function createHistorySummary(id: string, text: string) {
  return {
    id,
    text,
    provider: "Gemini",
    model: "1.5-pro",
    language: "en-US",
    durationMs: 42_000,
    copiedToClipboard: true,
    hasAudioFile: true,
    hasError: false,
    createdAt: "2026-04-28T12:00:00.000Z"
  };
}

function createHistoryEntry(id: string, text: string) {
  return {
    id,
    text,
    provider: "Gemini",
    model: "1.5-pro",
    language: "en-US",
    durationMs: 42_000,
    audioPath: `/tmp/${id}.wav`,
    audioDeleted: false,
    copiedToClipboard: true,
    error: null,
    createdAt: "2026-04-28T12:00:00.000Z"
  };
}

describe("+page integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetAppStores();

    pingMock.mockResolvedValue("pong");
    settingsMocks.loadAppSettings.mockResolvedValue(createDefaultAppSettings());
    settingsMocks.saveAppSettings.mockImplementation(async (settings) => settings);
    secretStoreMocks.hasGeminiApiKey.mockResolvedValue(false);
    secretStoreMocks.saveGeminiApiKey.mockResolvedValue(undefined);
    shortcutMocks.getRecordingShortcutStatus.mockResolvedValue({
      state: "unconfigured",
      source: "none",
      requestedShortcut: null,
      activeShortcut: null,
      detail: null
    });
    shortcutMocks.applyRecordingShortcut.mockResolvedValue({
      state: "active",
      source: "saved",
      requestedShortcut: "CommandOrControl+Alt+A",
      activeShortcut: "CommandOrControl+Alt+A",
      detail: null
    });
    recordingMocks.listRecordingInputDevices.mockResolvedValue([{ name: "USB Mic", isDefault: true }]);
    recordingMocks.getRecordingStatus.mockResolvedValue({
      phase: "idle",
      activeSessionId: null,
      inputDeviceName: null,
      elapsedMs: null,
      remainingMs: null,
      maxDurationMs: 900_000,
      limitReached: false,
      lastCompletedSessionId: null
    });
    historyMocks.getHistory.mockResolvedValue([
      createHistorySummary("entry-1", "First transcript line"),
      createHistorySummary("entry-2", "Second transcript line")
    ]);
    historyMocks.getTranscription.mockImplementation(async (id: string) =>
      id === "entry-1"
        ? createHistoryEntry("entry-1", "First transcript line\nFull transcript body")
        : createHistoryEntry("entry-2", "Second transcript line\nAnother transcript body")
    );
    historyMocks.clearHistory.mockResolvedValue({ deletedCount: 2 });
    historyMocks.deleteTranscription.mockResolvedValue({ deleted: true });
    transcriptionMocks.hasCompletedRecordingAudio.mockResolvedValue(true);
    transcriptionMocks.runCompletedRecordingTranscription.mockResolvedValue({
      transcript: {
        text: "Transcript ready",
        provider: "Gemini",
        model: "1.5-pro",
        language: "en-US",
        durationMs: 42_000
      },
      historyId: "entry-3",
      historySaved: true,
      historyError: null,
      copiedToClipboard: true,
      clipboardError: null,
      audioDeleted: false,
      audioDeleteError: null,
      retainedAudioPath: "/tmp/entry-3.wav"
    });
  });

  it("renders the SpeakEx shell and navigates from recording to settings", async () => {
    const user = userEvent.setup();

    render(Page);

    await screen.findByRole("heading", { name: "Where should we begin?" });

    expect(screen.getByRole("button", { name: "SpeakEx" })).toBeTruthy();
    expect(screen.getByText(/Gemini API key is missing/i)).toBeTruthy();

    await user.click(screen.getAllByRole("button", { name: "Settings" })[0]);

    expect(await screen.findByRole("heading", { name: "Settings" })).toBeTruthy();
    expect(screen.getByText(/Preferences are stored locally/i)).toBeTruthy();
  });

  it("loads history, shows transcript details, and clears the list", async () => {
    const user = userEvent.setup();

    render(Page);

    await screen.findByRole("heading", { name: "Where should we begin?" });
    await user.click(screen.getByRole("button", { name: "History" }));

    expect(await screen.findByRole("heading", { name: "History" })).toBeTruthy();

    const historyEntryButton = (await screen.findAllByRole("button", {
      name: /First transcript line/i
    }))[0];
    await user.click(historyEntryButton);

    expect(await screen.findByText(/Full transcript body/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Clear all" }));

    expect(await screen.findByText("No saved transcripts yet.")).toBeTruthy();
    expect(historyMocks.clearHistory).toHaveBeenCalledTimes(1);
  });

  it("submits the Gemini key and recording shortcut from settings", async () => {
    const user = userEvent.setup();

    render(Page);

    await screen.findByRole("heading", { name: "Where should we begin?" });
    await user.click(screen.getAllByRole("button", { name: "Settings" })[0]);

    const apiKeyInput = await screen.findByPlaceholderText("Enter API Key");
    await user.type(apiKeyInput, "api-key-123");
    await user.click(screen.getByRole("button", { name: "Save key" }));

    await waitFor(() => {
      expect(secretStoreMocks.saveGeminiApiKey).toHaveBeenCalledWith("api-key-123");
    });

    const shortcutInput = screen.getByPlaceholderText("e.g. CommandOrControl+Alt+A");
    await user.type(shortcutInput, "CommandOrControl+Alt+A");
    await user.click(screen.getByRole("button", { name: "Save and apply" }));

    await waitFor(() => {
      expect(settingsMocks.saveAppSettings).toHaveBeenCalledWith({
        ...createDefaultAppSettings(),
        shortcut: "CommandOrControl+Alt+A"
      });
    });
    expect(shortcutMocks.applyRecordingShortcut).toHaveBeenCalledWith("CommandOrControl+Alt+A");
  });
});
