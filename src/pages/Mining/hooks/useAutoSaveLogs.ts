import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const SAVE_INTERVAL_MS = 60_000;
const MAX_LOG_AGE_DAYS = 7;

/**
 * Auto-saves mining report to disk every 60s while mining (Tauri only).
 * Also saves a final snapshot when mining stops.
 * Cleans up log files older than 7 days on mount.
 *
 * Daily file format: JSON array of session snapshots.
 * Each session is identified by `session_id` (Redux sessionStart timestamp).
 * Multiple sessions per day are preserved; each session updates in place.
 */
export function useAutoSaveLogs(params: {
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
  const { enabled, sessionStart, buildReport } = params;
  const [lastSavePath, setLastSavePath] = useState<string | null>(null);
  const [lastSaveTime, setLastSaveTime] = useState<number | null>(null);
  const [dirReady, setDirReady] = useState(false);
  const logDirRef = useRef<string | null>(null);
  const prevEnabledRef = useRef(enabled);
  const buildReportRef = useRef(buildReport);
  buildReportRef.current = buildReport;
  const sessionStartRef = useRef(sessionStart);
  sessionStartRef.current = sessionStart;

  // Serialize saves — prevent concurrent read-modify-write
  const savingRef = useRef(false);

  // Resolve log directory on mount and clean old files
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const { appLogDir } = await import('@tauri-apps/api/path');
        const { mkdir, readDir, remove } = await import(
          '@tauri-apps/plugin-fs'
        );

        const baseDir = await appLogDir();
        const dir = baseDir.endsWith('/') ? `${baseDir}mining-logs` : `${baseDir}/mining-logs`;
        await mkdir(dir, { recursive: true });
        if (!cancelled) {
          logDirRef.current = dir;
          setDirReady(true);
        }

        // Clean logs older than 7 days
        try {
          const entries = await readDir(dir);
          const now = Date.now();
          const maxAge = MAX_LOG_AGE_DAYS * 24 * 60 * 60 * 1000;
          for (const entry of entries) {
            if (!entry.name || !entry.name.startsWith('mining-')) continue;
            const match = entry.name.match(
              /^mining-(\d{4}-\d{2}-\d{2})\.json$/
            );
            if (!match) continue;
            const fileDate = new Date(match[1]).getTime();
            if (Number.isNaN(fileDate)) continue;
            if (now - fileDate > maxAge) {
              try {
                await remove(`${dir}/${entry.name}`);
                console.log('[Mining] Cleaned old log:', entry.name);
              } catch {
                // ignore individual file errors
              }
            }
          }
        } catch {
          // readDir may fail if dir is new/empty — that's fine
        }
      } catch (err) {
        console.warn('[Mining] Failed to init auto-save dir:', err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Save report to daily file — returns file path on success
  const saveToDailyFile = useCallback(async (): Promise<string | null> => {
    const dir = logDirRef.current;
    if (!dir) return null;

    // Serialize: skip if another save is in progress
    if (savingRef.current) return lastSavePath;
    savingRef.current = true;

    try {
      const { writeTextFile, readTextFile } = await import(
        '@tauri-apps/plugin-fs'
      );
      const date = new Date().toISOString().slice(0, 10);
      const filePath = `${dir}/mining-${date}.json`;
      const report = buildReportRef.current() as Record<string, unknown>;
      const sid = String(sessionStartRef.current);
      report.session_id = sid;

      // Read existing sessions from today's file
      let sessions: Record<string, unknown>[] = [];
      try {
        const existing = await readTextFile(filePath);
        const parsed = JSON.parse(existing);
        if (Array.isArray(parsed)) {
          sessions = parsed;
        }
      } catch {
        // file doesn't exist yet or invalid — start fresh
      }

      // Replace this session's entry or append
      const idx = sessions.findIndex((s) => s.session_id === sid);
      if (idx >= 0) {
        sessions[idx] = report;
      } else {
        sessions.push(report);
      }

      await writeTextFile(filePath, JSON.stringify(sessions, null, 2));
      setLastSavePath(filePath);
      setLastSaveTime(Date.now());
      console.log(
        `[Mining] Auto-saved${sessions.length > 1 ? ` (${sessions.length} sessions)` : ''} to:`,
        filePath
      );
      return filePath;
    } catch (err) {
      console.warn('[Mining] Auto-save failed:', err);
      return null;
    } finally {
      savingRef.current = false;
    }
  }, [lastSavePath]);

  // Periodic save while enabled AND directory is ready
  useEffect(() => {
    if (!enabled || !dirReady) return;
    // Save immediately now that both conditions are met
    saveToDailyFile();
    const timer = setInterval(saveToDailyFile, SAVE_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [enabled, dirReady, saveToDailyFile]);

  // Save final snapshot when mining stops (enabled true -> false)
  useEffect(() => {
    if (prevEnabledRef.current && !enabled) {
      saveToDailyFile();
    }
    prevEnabledRef.current = enabled;
  }, [enabled, saveToDailyFile]);

  // Delete all log files from disk
  const clearAllLogs = useCallback(async () => {
    const dir = logDirRef.current;
    if (!dir) return;
    try {
      const { readDir, remove } = await import('@tauri-apps/plugin-fs');
      const entries = await readDir(dir);
      for (const entry of entries) {
        if (entry.name?.startsWith('mining-')) {
          try {
            await remove(`${dir}/${entry.name}`);
          } catch { /* ignore */ }
        }
      }
      setLastSavePath(null);
      setLastSaveTime(null);
      console.log('[Mining] Cleared all log files');
    } catch {
      // dir may not exist yet
    }
  }, []);

  const revealInFinder = useCallback((pathOverride?: string) => {
    const p = pathOverride || lastSavePath;
    if (!p) return;
    invoke('reveal_in_finder', { path: p }).catch((err) =>
      console.warn('[Mining] Reveal failed:', err)
    );
  }, [lastSavePath]);

  return { lastSavePath, lastSaveTime, saveToDailyFile, clearAllLogs, revealInFinder };
}
