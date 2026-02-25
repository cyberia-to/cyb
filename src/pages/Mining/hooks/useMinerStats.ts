import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';

function useMinerStats() {
  const { data, loading } = useQueryContract(LITIUM_MINE_CONTRACT, { stats: {} });

  const stats = data as
    | {
        total_proofs: number;
        total_rewards: string;
        unique_miners: number;
        avg_difficulty: number;
      }
    | undefined;

  return {
    uniqueMiners: stats?.unique_miners ?? 0,
    totalProofs: stats?.total_proofs ?? 0,
    avgDifficulty: stats?.avg_difficulty ?? 0,
    loading,
  };
}

export default useMinerStats;
