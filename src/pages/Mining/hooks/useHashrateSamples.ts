import { useRef, useMemo } from 'react';

const SAMPLE_INTERVAL_MS = 5000;

function useHashrateSamples(
  hashrate: number,
  isActive: boolean,
  maxSamples = 60
) {
  const samplesRef = useRef<number[]>([]);
  const prevActiveRef = useRef(isActive);
  const lastSampleTimeRef = useRef(0);

  // Clear samples when mining stops
  if (prevActiveRef.current && !isActive) {
    samplesRef.current = [];
    lastSampleTimeRef.current = 0;
  }
  prevActiveRef.current = isActive;

  // Downsample: only push a new sample every SAMPLE_INTERVAL_MS
  if (isActive && hashrate > 0) {
    const now = Date.now();
    if (now - lastSampleTimeRef.current >= SAMPLE_INTERVAL_MS) {
      lastSampleTimeRef.current = now;
      const arr = samplesRef.current;
      if (arr.length >= maxSamples) {
        arr.shift();
      }
      arr.push(hashrate);
    }
  }

  // Return stable reference — the array is mutated in place
  return useMemo(() => samplesRef.current, [isActive, hashrate]);
}

export default useHashrateSamples;
