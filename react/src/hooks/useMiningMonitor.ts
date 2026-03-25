import { useEffect, useRef } from 'react';
import { useDispatch } from 'react-redux';
import { setMiningStatus as setReduxMiningStatus } from 'src/redux/features/mining';
import type { MiningStatus } from 'src/redux/features/mining';
import { getWasmMiner } from 'src/pages/Mining/Mining';

const POLL_INTERVAL = 500;

const STOPPED: MiningStatus = {
  mining: false,
  hashrate: 0,
  total_hashes: 0,
  elapsed_secs: 0,
  pending_proofs: 0,
};

/**
 * App-level hook that keeps Redux mining state in sync with the mining backend.
 * Polls module-level WasmMiner directly.
 */
export default function useMiningMonitor() {
  const dispatch = useDispatch();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const wasActiveRef = useRef(false);

  useEffect(() => {
    const poll = async () => {
      try {
        const miner = getWasmMiner();
        if (miner) {
          const status = miner.getStatus();
          dispatch(setReduxMiningStatus(status));
          wasActiveRef.current = true;
        } else if (wasActiveRef.current) {
          // Miner was destroyed (stop) — clear Redux state
          dispatch(setReduxMiningStatus(STOPPED));
          wasActiveRef.current = false;
        }
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
