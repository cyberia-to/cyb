// TESTING: Coordinated mining countdown — remove after testing phase
import { useEffect, useRef, useState } from 'react';

/**
 * Returns remaining seconds until `targetTimeSec` (unix timestamp in seconds).
 * Updates every 1s. Returns 0 when target is in the past or undefined.
 */
function useCountdown(targetTimeSec: number | undefined): number {
  const calc = () => {
    if (!targetTimeSec) return 0;
    return Math.max(0, targetTimeSec - Math.floor(Date.now() / 1000));
  };

  const [remaining, setRemaining] = useState(calc);
  const prevTarget = useRef(targetTimeSec);

  // Synchronously reset when target changes (avoids stale-0 race)
  if (prevTarget.current !== targetTimeSec) {
    prevTarget.current = targetTimeSec;
    const v = calc();
    if (v !== remaining) {
      setRemaining(v);
    }
  }

  useEffect(() => {
    if (!targetTimeSec) {
      setRemaining(0);
      return;
    }

    const update = () => {
      const left = Math.max(0, targetTimeSec - Math.floor(Date.now() / 1000));
      setRemaining(left);
      return left;
    };

    const left = update();
    if (left <= 0) return;

    const timer = setInterval(() => {
      if (update() <= 0) {
        clearInterval(timer);
      }
    }, 1000);

    return () => clearInterval(timer);
  }, [targetTimeSec]);

  return remaining;
}

export default useCountdown;
