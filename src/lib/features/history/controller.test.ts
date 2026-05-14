import { describe, expect, it, vi, beforeEach } from "vitest";
import type { HistoryEntryViewModel } from "./types";
import type { HistoryTranscription } from "$lib/native/history";

const historyMocks = vi.hoisted(() => ({
  clearHistory: vi.fn(),
  deleteTranscription: vi.fn(),
  getHistory: vi.fn(),
  getTranscription: vi.fn()
}));

vi.mock("$lib/native/history", () => ({
  clearHistory: historyMocks.clearHistory,
  deleteTranscription: historyMocks.deleteTranscription,
  getHistory: historyMocks.getHistory,
  getTranscription: historyMocks.getTranscription
}));

import { createHistoryController } from "./controller";

function createViewModel(id: string, title = id) {
  return {
    id,
    title,
    excerpt: `${title} excerpt`,
    providerLabel: "Gemini",
    createdAtLabel: "Apr 28, 2026",
    durationLabel: "00:30",
    languageLabel: "en-US",
    clipboardLabel: "Copied to clipboard",
    audioLabel: "Audio retained",
    status: "saved" as const,
    statusLabel: "Saved"
  };
}

function createHistoryEntry(id: string, text = `${id} text`) {
  return {
    id,
    text,
    provider: "Gemini",
    model: "1.5-pro",
    language: "en-US",
    durationMs: 30_000,
    audioPath: `/tmp/${id}.wav`,
    audioDeleted: false,
    copiedToClipboard: true,
    error: null,
    createdAt: "2026-04-28T12:00:00.000Z"
  };
}

function createControllerHarness() {
  const state = {
    historyState: "loading" as "loading" | "ready" | "error",
    historyEntries: [] as HistoryEntryViewModel[],
    historyError: "",
    historyDetailState: "idle" as "idle" | "loading" | "ready" | "error",
    historyDetailError: "",
    historyBusyEntryId: null as string | null,
    isClearingHistory: false,
    selectedHistoryEntryId: null as string | null,
    selectedHistoryEntry: null as HistoryTranscription | null
  };

  const controller = createHistoryController({
    getHistoryState: () => state.historyState,
    setHistoryState: (value) => {
      state.historyState = value;
    },
    setHistoryEntries: (value) => {
      state.historyEntries = value;
    },
    getHistoryEntries: () => state.historyEntries,
    setHistoryError: (value) => {
      state.historyError = value;
    },
    setHistoryDetailState: (value) => {
      state.historyDetailState = value;
    },
    getHistoryDetailState: () => state.historyDetailState,
    setHistoryDetailError: (value) => {
      state.historyDetailError = value;
    },
    setHistoryBusyEntryId: (value) => {
      state.historyBusyEntryId = value;
    },
    getHistoryBusyEntryId: () => state.historyBusyEntryId,
    setIsClearingHistory: (value) => {
      state.isClearingHistory = value;
    },
    getIsClearingHistory: () => state.isClearingHistory,
    setSelectedHistoryEntryId: (value) => {
      state.selectedHistoryEntryId = value;
    },
    getSelectedHistoryEntryId: () => state.selectedHistoryEntryId,
    setSelectedHistoryEntry: (value) => {
      state.selectedHistoryEntry = value;
    },
    getSelectedHistoryEntry: () => state.selectedHistoryEntry,
    mapHistoryEntry: (entry) => createViewModel(entry.id, entry.text),
    reportErrorLog: vi.fn().mockResolvedValue(undefined)
  });

  return { controller, state };
}

describe("createHistoryController", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads history and selects the preferred entry when it exists", async () => {
    historyMocks.getHistory.mockResolvedValue([
      {
        id: "entry-1",
        text: "First entry",
        provider: "Gemini",
        model: "1.5-pro",
        language: "en-US",
        durationMs: 10_000,
        copiedToClipboard: true,
        hasAudioFile: true,
        hasError: false,
        createdAt: "2026-04-28T12:00:00.000Z"
      },
      {
        id: "entry-2",
        text: "Second entry",
        provider: "Gemini",
        model: "1.5-pro",
        language: "pt-BR",
        durationMs: 20_000,
        copiedToClipboard: false,
        hasAudioFile: false,
        hasError: false,
        createdAt: "2026-04-28T12:05:00.000Z"
      }
    ]);
    historyMocks.getTranscription.mockResolvedValue(createHistoryEntry("entry-2", "Second entry"));

    const { controller, state } = createControllerHarness();

    await controller.loadHistoryEntries("entry-2");

    expect(state.historyState).toBe("ready");
    expect(state.historyEntries.map((entry) => entry.id)).toEqual(["entry-1", "entry-2"]);
    expect(state.selectedHistoryEntryId).toBe("entry-2");
    expect(state.selectedHistoryEntry?.text).toBe("Second entry");
    expect(state.historyDetailState).toBe("ready");
  });

  it("falls forward to the next entry after deleting the current selection", async () => {
    historyMocks.deleteTranscription.mockResolvedValue({ deleted: true });
    historyMocks.getTranscription.mockResolvedValue(createHistoryEntry("entry-3", "Third entry"));

    const { controller, state } = createControllerHarness();
    state.historyState = "ready";
    state.historyEntries = [
      createViewModel("entry-1", "First entry"),
      createViewModel("entry-2", "Second entry"),
      createViewModel("entry-3", "Third entry")
    ];
    state.selectedHistoryEntryId = "entry-2";
    state.selectedHistoryEntry = createHistoryEntry("entry-2", "Second entry");
    state.historyDetailState = "ready";

    await controller.removeHistoryEntry("entry-2");

    expect(state.historyEntries.map((entry) => entry.id)).toEqual(["entry-1", "entry-3"]);
    expect(state.selectedHistoryEntryId).toBe("entry-3");
    expect(state.selectedHistoryEntry?.id).toBe("entry-3");
    expect(state.historyBusyEntryId).toBeNull();
  });

  it("clears all history and resets the current selection", async () => {
    historyMocks.clearHistory.mockResolvedValue({ deletedCount: 2 });

    const { controller, state } = createControllerHarness();
    state.historyState = "ready";
    state.historyEntries = [createViewModel("entry-1"), createViewModel("entry-2")];
    state.selectedHistoryEntryId = "entry-1";
    state.selectedHistoryEntry = createHistoryEntry("entry-1");
    state.historyDetailState = "ready";

    await controller.clearAllHistory();

    expect(state.historyEntries).toEqual([]);
    expect(state.selectedHistoryEntryId).toBeNull();
    expect(state.selectedHistoryEntry).toBeNull();
    expect(state.historyDetailState).toBe("idle");
    expect(state.historyState).toBe("ready");
  });

  it("surfaces a detail error when the selected entry no longer exists", async () => {
    historyMocks.getTranscription.mockResolvedValue(null);

    const { controller, state } = createControllerHarness();

    await controller.selectHistoryEntry("missing-entry");

    expect(state.selectedHistoryEntryId).toBe("missing-entry");
    expect(state.selectedHistoryEntry).toBeNull();
    expect(state.historyDetailState).toBe("error");
    expect(state.historyDetailError).toContain("no longer available");
  });
});
