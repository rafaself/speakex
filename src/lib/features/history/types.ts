export type HistoryEntryStatus = "saved" | "attention";

export interface HistoryEntryViewModel {
  id: string;
  title: string;
  excerpt: string;
  providerLabel: string;
  createdAtLabel: string;
  durationLabel: string;
  languageLabel: string;
  clipboardLabel: string;
  audioLabel: string;
  status: HistoryEntryStatus;
  statusLabel: string;
}
