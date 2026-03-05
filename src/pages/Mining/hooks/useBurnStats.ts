import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_CORE_CONTRACT } from 'src/constants/mining';
import type { BurnStatsResponse } from 'src/generated/lithium/LitiumCore.types';

function useBurnStats() {
  const { data, loading, refetch } = useQueryContract(LITIUM_CORE_CONTRACT, {
    burn_stats: {},
  });

  const stats = data as BurnStatsResponse | undefined;

  return {
    burnStats: stats,
    loading,
    refetch,
  };
}

export default useBurnStats;
