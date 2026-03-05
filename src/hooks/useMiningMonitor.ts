import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useDispatch } from 'react-redux';
import { isTauri } from 'src/utils/tauri';
import { setMiningStatus as setReduxMiningStatus } from 'src/redux/features/mining';
import type { MiningStatus } from 'src/redux/features/mining';

const POLL_INTERVAL = 1000;

/**
 * App-level hook that keeps Redux mining state in sync with the Tauri backend.
 * Single source of truth for MiningStatus — the Mining page reads from Redux.
 */
export default function useMiningMonitor() {
  const dispatch = useDispatch();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const snapshotsRef = useRef<{ time: number; hashes: number }[]>([]);

  useEffect(() => {
    if (!isTauri()) return;

    const poll = async () => {
      try {
        const raw = (await invoke('get_mining_status')) as MiningStatus;

        // 30s rolling hashrate (Rust reports lifetime average)
        let hashrate = raw.hashrate;
        if (raw.mining) {
          const now = Date.now();
          const cutoff = now - 30_000;
          while (snapshotsRef.current.length > 0 && snapshotsRef.current[0].time < cutoff) {
            snapshotsRef.current.shift();
          }
          snapshotsRef.current.push({ time: now, hashes: raw.total_hashes });

          if (snapshotsRef.current.length >= 2) {
            const oldest = snapshotsRef.current[0];
            const newest = snapshotsRef.current[snapshotsRef.current.length - 1];
            const dt = (newest.time - oldest.time) / 1000;
            if (dt > 0.5) {
              hashrate = (newest.hashes - oldest.hashes) / dt;
            }
          }
        } else {
          snapshotsRef.current = [];
        }

        dispatch(setReduxMiningStatus({ ...raw, hashrate }));
      } catch {
        // Backend not available
      }
    };

    poll();
    timerRef.current = setInterval(poll, POLL_INTERVAL);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [dispatch]);
}
