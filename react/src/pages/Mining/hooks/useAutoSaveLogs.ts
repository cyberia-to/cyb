/**
 * Previously auto-saved mining reports to disk via Tauri filesystem APIs.
 * Now a no-op stub — the hook interface is preserved for callers.
 */
export function useAutoSaveLogs(_params: {
  enabled: boolean;
  sessionStart: number;
  buildReport: () => object;
}): {
  lastSavePath: string | null;
  lastSaveTime: number | null;
  saveToDailyFile: () => Promise<string | null>;
  clearAllLogs: () => Promise<void>;
  revealInFinder: () => void;
} {
  return {
    lastSavePath: null,
    lastSaveTime: null,
    saveToDailyFile: async () => null,
    clearAllLogs: async () => {},
    revealInFinder: () => {},
  };
}
