import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ErrorLogEntry } from "$lib/native/logs";

const logsMocks = vi.hoisted(() => ({
  clearErrorLogs: vi.fn(),
  getErrorLogs: vi.fn()
}));

vi.mock("$lib/native/logs", () => ({
  clearErrorLogs: logsMocks.clearErrorLogs,
  getErrorLogs: logsMocks.getErrorLogs
}));

import { createLogsController } from "./controller";

function createLog(id: string, summary = "Something failed"): ErrorLogEntry {
  return {
    id,
    scope: "transcription",
    source: "frontend",
    summary,
    detail: "Gemini API returned 400 Bad Request",
    createdAt: "2026-05-14T12:00:00.000Z"
  };
}

function createControllerHarness() {
  const state = {
    logsState: "loading" as "loading" | "ready" | "error",
    logsEntries: [] as ErrorLogEntry[],
    logsError: "",
    isClearingLogs: false
  };

  const controller = createLogsController({
    setLogsState: (value) => {
      state.logsState = value;
    },
    setLogsEntries: (value) => {
      state.logsEntries = value;
    },
    getLogsEntries: () => state.logsEntries,
    setLogsError: (value) => {
      state.logsError = value;
    },
    setIsClearingLogs: (value) => {
      state.isClearingLogs = value;
    },
    getIsClearingLogs: () => state.isClearingLogs
  });

  return { controller, state };
}

describe("createLogsController", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads persisted error logs", async () => {
    logsMocks.getErrorLogs.mockResolvedValue([createLog("log-1"), createLog("log-2")]);

    const { controller, state } = createControllerHarness();

    await controller.loadErrorLogs();

    expect(state.logsState).toBe("ready");
    expect(state.logsEntries.map((entry) => entry.id)).toEqual(["log-1", "log-2"]);
  });

  it("clears persisted error logs", async () => {
    logsMocks.clearErrorLogs.mockResolvedValue({ deletedCount: 2 });

    const { controller, state } = createControllerHarness();
    state.logsState = "ready";
    state.logsEntries = [createLog("log-1"), createLog("log-2")];

    await controller.clearAllErrorLogs();

    expect(state.logsEntries).toEqual([]);
    expect(state.logsState).toBe("ready");
    expect(state.isClearingLogs).toBe(false);
  });
});