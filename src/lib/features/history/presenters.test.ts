import { describe, expect, it } from "vitest";

import {
  createHistoryExcerpt,
  createHistoryTitle,
  formatCreatedAt,
  mapHistoryEntry
} from "./presenters";

describe("history presenters", () => {
  it("builds a fallback title for empty transcripts", () => {
    expect(createHistoryTitle("   ")).toBe("Untitled transcript");
  });

  it("truncates long titles from the first transcript line", () => {
    expect(
      createHistoryTitle("This is a very long transcript title that should be trimmed at some point\nNext line")
    ).toBe("This is a very long transcript title that should be t…");
  });

  it("normalizes whitespace when building excerpts", () => {
    expect(createHistoryExcerpt("Hello   world\n\nfrom   SpeakEx")).toBe("Hello world from SpeakEx");
  });

  it("returns the original value when the created-at date is invalid", () => {
    expect(formatCreatedAt("not-a-date")).toBe("not-a-date");
  });

  it("maps a history summary into the UI view model", () => {
    expect(
      mapHistoryEntry({
        id: "entry-1",
        text: "First line\nSecond line",
        provider: "Gemini",
        model: "1.5-pro",
        language: null,
        durationMs: 65_000,
        copiedToClipboard: false,
        hasAudioFile: true,
        hasError: true,
        createdAt: "2026-04-28T12:00:00.000Z"
      })
    ).toMatchObject({
      id: "entry-1",
      title: "First line",
      excerpt: "First line Second line",
      providerLabel: "Gemini · 1.5-pro",
      durationLabel: "01:05",
      languageLabel: "Auto / unspecified",
      clipboardLabel: "Not copied to clipboard",
      audioLabel: "Audio retained",
      status: "attention",
      statusLabel: "Saved with warnings"
    });
  });
});
