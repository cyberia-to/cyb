import { useRef, useState, useEffect } from 'react';

function useHashrateSamples(
  hashrate: number,
  isActive: boolean,
  maxSamples = 60
) {
  const samplesRef = useRef<number[]>([]);
  const [, setTick] = useState(0);

  useEffect(() => {
    if (!isActive) return;
    if (hashrate <= 0) return;

    samplesRef.current = [...samplesRef.current.slice(-(maxSamples - 1)), hashrate];
    setTick((t) => t + 1);
  }, [hashrate, isActive, maxSamples]);

  useEffect(() => {
    if (!isActive) {
      samplesRef.current = [];
      setTick((t) => t + 1);
    }
  }, [isActive]);

  return samplesRef.current;
}

export default useHashrateSamples;
