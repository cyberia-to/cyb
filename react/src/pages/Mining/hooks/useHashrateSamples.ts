import { useRef, useEffect } from 'react';

const SAMPLE_INTERVAL_MS = 5000;

function useHashrateSamples(
  hashrate: number,
  isActive: boolean,
  maxSamples = 60
) {
  const samplesRef = useRef<number[]>([]);
  const hashrateRef = useRef(hashrate);
  hashrateRef.current = hashrate;

  useEffect(() => {
    if (!isActive) {
      samplesRef.current = [];
      return;
    }

    const interval = setInterval(() => {
      const hr = hashrateRef.current;
      if (hr > 0) {
        const arr = samplesRef.current;
        if (arr.length >= maxSamples) {
          arr.shift();
        }
        arr.push(hr);
      }
    }, SAMPLE_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [isActive, maxSamples]);

  return samplesRef.current;
}

export default useHashrateSamples;
