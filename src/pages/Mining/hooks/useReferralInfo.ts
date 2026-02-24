import useQueryContract from 'src/hooks/contract/useQueryContract';
import { UHASH_CONTRACT } from 'src/constants/mining';
import type { LithiumReferralInfoResponse } from 'src/types/miningProofTx';

function useReferralInfo(address: string | undefined) {
  const { data, loading } = useQueryContract(
    UHASH_CONTRACT,
    address
      ? { lithium_referral_info: { address } }
      : { lithium_emission_info: {} } // dummy query when no address
  );

  const info =
    address && data ? (data as LithiumReferralInfoResponse) : undefined;

  return {
    referralInfo: info,
    loading,
  };
}

export default useReferralInfo;
