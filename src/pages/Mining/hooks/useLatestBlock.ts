import { useEffect, useRef, useState } from 'react';
import { RPC_URL } from 'src/constants/config';

type BlockInfo = {
  blockHash: string;
  dataHash: string;
  height: number;
  timestamp: number; // unix seconds from block header
};

const REFETCH_INTERVAL_MS = 6_000; // ~1 Bostrom block

function useLatestBlock(): BlockInfo | undefined {
  const [block, setBlock] = useState<BlockInfo | undefined>();
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    async function fetchBlock() {
      try {
        const url = `${RPC_URL}/block`;
        console.log('[useLatestBlock] Fetching:', url);
        const res = await fetch(url);
        const json = await res.json();
        const result = json.result ?? json;

        const blockHash: string =
          result.block_id?.hash ?? '';
        const dataHash: string =
          result.block?.header?.data_hash ?? '';
        const height: number = Number(
          result.block?.header?.height ?? 0
        );
        const timeStr: string = result.block?.header?.time ?? '';
        const timestamp = timeStr
          ? Math.floor(new Date(timeStr).getTime() / 1000)
          : 0;

        if (blockHash && height > 0 && timestamp > 0) {
          setBlock({ blockHash, dataHash, height, timestamp });
        }
      } catch (err) {
        console.error('[useLatestBlock] fetch error:', err);
      }
    }

    fetchBlock();
    intervalRef.current = setInterval(fetchBlock, REFETCH_INTERVAL_MS);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);

  return block;
}

export default useLatestBlock;
