import { useRef, useMemo } from 'react';

function useHashrateSamples(
  hashrate: number,
  isActive: boolean,
  maxSamples = 60
) {
  const samplesRef = useRef<number[]>([]);
  const prevActiveRef = useRef(isActive);

  // Clear samples when mining stops
  if (prevActiveRef.current && !isActive) {
    samplesRef.current = [];
  }
  prevActiveRef.current = isActive;

  // Append sample during render (no setTick / forced re-render needed —
  // the parent already re-renders when hashrate changes)
  if (isActive && hashrate > 0) {
    const arr = samplesRef.current;
    if (arr.length >= maxSamples) {
      arr.shift();
    }
    arr.push(hashrate);
  }

  // Return stable reference — the array is mutated in place
  return useMemo(() => samplesRef.current, [isActive, hashrate]);
}

export default useHashrateSamples;
