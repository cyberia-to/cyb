import useQueryContract from 'src/hooks/contract/useQueryContract';
import { UHASH_CONTRACT } from 'src/constants/mining';
import type { BurnStatsResponse } from 'src/types/miningProofTx';

function useBurnStats() {
  const { data, loading } = useQueryContract(UHASH_CONTRACT, {
    burn_stats: {},
  });

  const stats = data as BurnStatsResponse | undefined;

  return {
    burnStats: stats,
    loading,
  };
}

export default useBurnStats;
