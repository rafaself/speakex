import type { HistoryTranscriptionSummary } from "$lib/native/history";

import type { HistoryEntryViewModel } from "./types";

export function createHistoryTitle(text: string): string {
  const trimmedText = text.trim();

  if (!trimmedText) {
    return "Untitled transcript";
  }

  const firstLine = trimmedText.split(/\r?\n/u, 1)[0] ?? trimmedText;

  return firstLine.length > 56 ? `${firstLine.slice(0, 53).trimEnd()}…` : firstLine;
}

export function createHistoryExcerpt(text: string): string {
  const normalizedText = text.replace(/\s+/gu, " ").trim();

  if (!normalizedText) {
    return "Saved transcript text is empty.";
  }

  return normalizedText.length > 180 ? `${normalizedText.slice(0, 177).trimEnd()}…` : normalizedText;
}

export function formatCreatedAt(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.valueOf())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(date);
}

export function mapHistoryEntry(entry: HistoryTranscriptionSummary): HistoryEntryViewModel {
  return {
    id: entry.id,
    title: createHistoryTitle(entry.text),
    excerpt: createHistoryExcerpt(entry.text),
    providerLabel: [entry.provider, entry.model].filter(Boolean).join(" · ") || "Unknown provider",
    createdAtLabel: formatCreatedAt(entry.createdAt),
    durationLabel: entry.durationMs === null ? "—" : formatHistoryDuration(entry.durationMs),
    languageLabel: entry.language ?? "Auto / unspecified",
    clipboardLabel: entry.copiedToClipboard ? "Copied to clipboard" : "Not copied to clipboard",
    audioLabel: entry.hasAudioFile ? "Audio retained" : "No retained audio",
    status: entry.hasError ? "attention" : "saved",
    statusLabel: entry.hasError ? "Saved with warnings" : "Saved"
  };
}

function formatHistoryDuration(durationMs: number): string {
  const totalSeconds = Math.round(durationMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }

  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
