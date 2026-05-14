import {
  clearHistory,
  deleteTranscription,
  getHistory,
  getTranscription,
  type HistoryTranscription
} from "$lib/native/history";

import type { HistoryEntryViewModel } from "./types";

interface HistoryControllerContext {
  getHistoryState: () => "loading" | "ready" | "error";
  setHistoryState: (state: "loading" | "ready" | "error") => void;
  setHistoryEntries: (entries: HistoryEntryViewModel[]) => void;
  getHistoryEntries: () => HistoryEntryViewModel[];
  setHistoryError: (message: string) => void;
  setHistoryDetailState: (state: "idle" | "loading" | "ready" | "error") => void;
  getHistoryDetailState: () => "idle" | "loading" | "ready" | "error";
  setHistoryDetailError: (message: string) => void;
  setHistoryBusyEntryId: (id: string | null) => void;
  getHistoryBusyEntryId: () => string | null;
  setIsClearingHistory: (value: boolean) => void;
  getIsClearingHistory: () => boolean;
  setSelectedHistoryEntryId: (id: string | null) => void;
  getSelectedHistoryEntryId: () => string | null;
  setSelectedHistoryEntry: (entry: HistoryTranscription | null) => void;
  getSelectedHistoryEntry: () => HistoryTranscription | null;
  mapHistoryEntry: (entry: Awaited<ReturnType<typeof getHistory>>[number]) => HistoryEntryViewModel;
  reportErrorLog: (entry: { scope: string; summary: string; detail: string }) => Promise<void>;
}

export function createHistoryController(context: HistoryControllerContext) {
  let latestHistoryDetailRequest = 0;

  function reportHistoryError(summary: string, detail: string) {
    void context.reportErrorLog({
      scope: "history",
      summary,
      detail
    });
  }

  function clearHistorySelection() {
    latestHistoryDetailRequest += 1;
    context.setSelectedHistoryEntryId(null);
    context.setSelectedHistoryEntry(null);
    context.setHistoryDetailError("");
    context.setHistoryDetailState("idle");
  }

  async function selectHistoryEntry(id: string) {
    if (
      context.getSelectedHistoryEntryId() === id &&
      (context.getHistoryDetailState() === "loading" ||
        (context.getHistoryDetailState() === "ready" &&
          context.getSelectedHistoryEntry()?.id === id))
    ) {
      return;
    }

    context.setSelectedHistoryEntryId(id);
    context.setSelectedHistoryEntry(null);
    context.setHistoryDetailError("");
    context.setHistoryDetailState("loading");

    const requestId = ++latestHistoryDetailRequest;

    try {
      const entry = await getTranscription(id);

      if (requestId !== latestHistoryDetailRequest) {
        return;
      }

      if (entry === null) {
        throw new Error("The selected transcript is no longer available in local history.");
      }

      context.setSelectedHistoryEntry(entry);
      context.setHistoryDetailState("ready");
    } catch (error) {
      if (requestId !== latestHistoryDetailRequest) {
        return;
      }

      const detail =
        error instanceof Error ? error.message : "Unable to load the selected transcript";

      context.setSelectedHistoryEntry(null);
      context.setHistoryDetailError(detail);
      context.setHistoryDetailState("error");
      reportHistoryError("History detail could not be loaded.", detail);
    }
  }

  async function loadHistoryEntries(preferredSelectionId: string | null = null) {
    context.setHistoryState("loading");
    context.setHistoryError("");

    try {
      const historyEntries = (await getHistory()).map(context.mapHistoryEntry);
      context.setHistoryEntries(historyEntries);
      context.setHistoryState("ready");

      const selectedHistoryEntryId = context.getSelectedHistoryEntryId();
      const nextSelectedId =
        preferredSelectionId !== null && historyEntries.some((entry) => entry.id === preferredSelectionId)
          ? preferredSelectionId
          : selectedHistoryEntryId !== null &&
              historyEntries.some((entry) => entry.id === selectedHistoryEntryId)
            ? selectedHistoryEntryId
            : historyEntries[0]?.id ?? null;

      if (nextSelectedId === null) {
        clearHistorySelection();
        return;
      }

      await selectHistoryEntry(nextSelectedId);
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unable to load saved transcripts";
      context.setHistoryEntries([]);
      clearHistorySelection();
      context.setHistoryError(detail);
      context.setHistoryState("error");
      reportHistoryError("Transcript history could not be loaded.", detail);
    }
  }

  async function removeHistoryEntry(id: string) {
    if (context.getHistoryBusyEntryId() || context.getIsClearingHistory()) {
      return;
    }

    context.setHistoryBusyEntryId(id);
    context.setHistoryError("");

    try {
      const result = await deleteTranscription(id);

      if (!result.deleted) {
        throw new Error("The selected transcript was not found in local history.");
      }

      const currentEntries = context.getHistoryEntries();
      const removedIndex = currentEntries.findIndex((entry) => entry.id === id);
      const nextEntries = currentEntries.filter((entry) => entry.id !== id);
      context.setHistoryEntries(nextEntries);

      if (context.getSelectedHistoryEntryId() === id) {
        const fallbackEntry =
          nextEntries[removedIndex] ?? nextEntries[Math.max(removedIndex - 1, 0)] ?? null;

        if (fallbackEntry) {
          await selectHistoryEntry(fallbackEntry.id);
        } else {
          clearHistorySelection();
        }
      }

      context.setHistoryState("ready");
    } catch (error) {
      const detail =
        error instanceof Error ? error.message : "Unable to delete the selected transcript";
      context.setHistoryError(detail);
      reportHistoryError("A transcript could not be deleted from history.", detail);
    } finally {
      context.setHistoryBusyEntryId(null);
    }
  }

  async function clearAllHistory() {
    if (
      context.getIsClearingHistory() ||
      context.getHistoryEntries().length === 0 ||
      context.getHistoryBusyEntryId()
    ) {
      return;
    }

    context.setIsClearingHistory(true);
    context.setHistoryError("");

    try {
      await clearHistory();
      context.setHistoryEntries([]);
      clearHistorySelection();
      context.setHistoryState("ready");
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unable to clear transcript history";
      context.setHistoryError(detail);
      reportHistoryError("Transcript history could not be cleared.", detail);
    } finally {
      context.setIsClearingHistory(false);
    }
  }

  return {
    clearAllHistory,
    loadHistoryEntries,
    removeHistoryEntry,
    selectHistoryEntry
  };
}
