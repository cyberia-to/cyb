import { useQuery } from '@tanstack/react-query';
import { RPC_URL } from 'src/constants/config';

type BlockInfo = {
  blockHash: string;
  dataHash: string;
  height: number;
  timestamp: number; // unix seconds from block header
};

async function fetchLatestBlock(): Promise<BlockInfo | undefined> {
  const url = `${RPC_URL}/block`;
  const res = await fetch(url);
  const json = await res.json();
  const result = json.result ?? json;

  const blockHash: string = result.block_id?.hash ?? '';
  const dataHash: string = result.block?.header?.data_hash ?? '';
  const height: number = Number(result.block?.header?.height ?? 0);
  const timeStr: string = result.block?.header?.time ?? '';
  const timestamp = timeStr
    ? Math.floor(new Date(timeStr).getTime() / 1000)
    : 0;

  if (blockHash && height > 0 && timestamp > 0) {
    return { blockHash, dataHash, height, timestamp };
  }
  return undefined;
}

function useLatestBlock() {
  const { data, refetch } = useQuery(['latestBlock'], fetchLatestBlock, {
    staleTime: Infinity,
    keepPreviousData: true,
  });

  return { block: data, refetchBlock: refetch };
}

export default useLatestBlock;
