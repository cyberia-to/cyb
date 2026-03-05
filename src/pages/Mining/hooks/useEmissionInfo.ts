import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import type { EmissionInfoResponse } from 'src/generated/lithium/LitiumMine.types';

function useEmissionInfo() {
  const { data, loading, refetch } = useQueryContract(LITIUM_MINE_CONTRACT, {
    emission_info: {},
  });

  const emission = data as EmissionInfoResponse | undefined;

  return {
    emission,
    loading,
    refetch,
  };
}

export default useEmissionInfo;
