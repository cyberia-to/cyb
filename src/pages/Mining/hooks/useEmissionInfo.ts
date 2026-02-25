import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import type { LithiumEmissionInfoResponse } from 'src/types/miningProofTx';

function useEmissionInfo() {
  const { data, loading } = useQueryContract(LITIUM_MINE_CONTRACT, {
    lithium_emission_info: {},
  });

  const emission = data as LithiumEmissionInfoResponse | undefined;

  return {
    emission,
    loading,
  };
}

export default useEmissionInfo;
