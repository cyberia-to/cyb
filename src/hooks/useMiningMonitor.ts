import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useDispatch } from 'react-redux';
import { isTauri } from 'src/utils/tauri';
import { setMiningStatus as setReduxMiningStatus } from 'src/redux/features/mining';
import type { MiningStatus } from 'src/redux/features/mining';

const POLL_INTERVAL = 1000;

/**
 * App-level hook that keeps Redux mining state in sync with the Tauri backend.
 * Uses the backend's 30-second rolling hashrate directly — no frontend re-derivation.
 */
export default function useMiningMonitor() {
  const dispatch = useDispatch();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const poll = async () => {
      try {
        const raw = (await invoke('get_mining_status')) as MiningStatus;
        dispatch(setReduxMiningStatus(raw));
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
