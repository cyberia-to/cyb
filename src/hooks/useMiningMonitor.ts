import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useDispatch } from 'react-redux';
import { isTauri } from 'src/utils/tauri';
import { setMiningStatus as setReduxMiningStatus } from 'src/redux/features/mining';
import type { MiningStatus } from 'src/redux/features/mining';

const POLL_INTERVAL = 1000;
// EMA smoothing factor — lower = smoother (0.1 ≈ 10-second effective window)
const EMA_ALPHA = 0.1;

/**
 * App-level hook that keeps Redux mining state in sync with the Tauri backend.
 * Single source of truth for MiningStatus — the Mining page reads from Redux.
 */
export default function useMiningMonitor() {
  const dispatch = useDispatch();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const prevHashesRef = useRef<number | null>(null);
  const emaRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const poll = async () => {
      try {
        const raw = (await invoke('get_mining_status')) as MiningStatus;

        let hashrate = raw.hashrate;
        if (raw.mining && prevHashesRef.current !== null) {
          const dt = POLL_INTERVAL / 1000;
          const instantRate = (raw.total_hashes - prevHashesRef.current) / dt;
          if (emaRef.current === null) {
            emaRef.current = instantRate;
          } else {
            emaRef.current =
              EMA_ALPHA * instantRate + (1 - EMA_ALPHA) * emaRef.current;
          }
          hashrate = emaRef.current;
        }

        if (raw.mining) {
          prevHashesRef.current = raw.total_hashes;
        } else {
          prevHashesRef.current = null;
          emaRef.current = null;
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
