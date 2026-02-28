import { useCallback, useEffect, useRef, useState } from 'react';
import { LITIUM_STAKE_CONTRACT } from 'src/constants/mining';
import { useQueryClient as useCyberQueryClient } from 'src/contexts/queryClient';
import type { LithiumStakeInfoResponse } from 'src/types/miningProofTx';

const POLL_INTERVAL = 15_000;

function useStakeInfo(address: string | undefined) {
  const queryClient = useCyberQueryClient();
  const [stakeInfo, setStakeInfo] = useState<LithiumStakeInfoResponse | undefined>();
  const [loading, setLoading] = useState(false);
  const [counter, setCounter] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Store latest fetch function in a ref to avoid interval churn
  const fetchRef = useRef<() => void>(() => {});

  const fetchStakeInfo = useCallback(async () => {
    if (!queryClient || !address) {
      setStakeInfo(undefined);
      return;
    }
    setLoading(true);
    try {
      const data = await queryClient.queryContractSmart(
        LITIUM_STAKE_CONTRACT,
        { stake_info: { address } }
      );
      setStakeInfo(data as LithiumStakeInfoResponse);
    } catch {
      // contract may not have entry for this address yet
      setStakeInfo(undefined);
    } finally {
      setLoading(false);
    }
  }, [queryClient, address]);

  fetchRef.current = fetchStakeInfo;

  useEffect(() => {
    fetchStakeInfo();
  }, [fetchStakeInfo, counter]);

  // Periodic polling disabled — manual refetch via refetch() callback
  void intervalRef;

  const refetch = useCallback(() => {
    setCounter((c) => c + 1);
  }, []);

  return {
    stakeInfo,
    loading,
    refetch,
  };
}

export default useStakeInfo;
