import useQueryContract from 'src/hooks/contract/useQueryContract';
import { UHASH_CONTRACT } from 'src/constants/mining';
import type { LithiumStakeInfoResponse } from 'src/types/miningProofTx';

function useStakeInfo(address: string | undefined) {
  const { data, loading } = useQueryContract(
    UHASH_CONTRACT,
    address
      ? { lithium_stake_info: { address } }
      : { lithium_emission_info: {} } // dummy query when no address
  );

  const info =
    address && data ? (data as LithiumStakeInfoResponse) : undefined;

  return {
    stakeInfo: info,
    loading,
  };
}

export default useStakeInfo;
