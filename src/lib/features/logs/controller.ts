import { clearErrorLogs, getErrorLogs, type ErrorLogEntry } from "$lib/native/logs";

interface LogsControllerContext {
  setLogsState: (state: "loading" | "ready" | "error") => void;
  setLogsEntries: (entries: ErrorLogEntry[]) => void;
  getLogsEntries: () => ErrorLogEntry[];
  setLogsError: (message: string) => void;
  setIsClearingLogs: (value: boolean) => void;
  getIsClearingLogs: () => boolean;
}

export function createLogsController(context: LogsControllerContext) {
  async function loadErrorLogs() {
    context.setLogsState("loading");
    context.setLogsError("");

    try {
      context.setLogsEntries(await getErrorLogs());
      context.setLogsState("ready");
    } catch (error) {
      context.setLogsEntries([]);
      context.setLogsError(error instanceof Error ? error.message : "Unable to load error logs.");
      context.setLogsState("error");
    }
  }

  async function clearAllErrorLogs() {
    if (context.getIsClearingLogs() || context.getLogsEntries().length === 0) {
      return;
    }

    context.setIsClearingLogs(true);
    context.setLogsError("");

    try {
      await clearErrorLogs();
      context.setLogsEntries([]);
      context.setLogsState("ready");
    } catch (error) {
      context.setLogsError(error instanceof Error ? error.message : "Unable to clear error logs.");
    } finally {
      context.setIsClearingLogs(false);
    }
  }

  return {
    clearAllErrorLogs,
    loadErrorLogs
  };
}