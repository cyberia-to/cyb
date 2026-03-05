import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import type { StatsResponse } from 'src/generated/lithium/LitiumMine.types';

function useMinerStats() {
  const { data, loading, refetch } = useQueryContract(LITIUM_MINE_CONTRACT, { stats: {} });

  const stats = data as StatsResponse | undefined;

  return {
    uniqueMiners: stats?.unique_miners ?? 0,
    totalProofs: stats?.total_proofs ?? 0,
    avgDifficulty: stats?.avg_difficulty ?? 0,
    loading,
    refetch,
  };
}

export default useMinerStats;
