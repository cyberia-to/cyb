import { useCallback, useEffect, useRef, useState } from 'react';
import { LITIUM_REFER_CONTRACT } from 'src/constants/mining';
import { useQueryClient as useCyberQueryClient } from 'src/contexts/queryClient';
import type { LithiumReferralInfoResponse } from 'src/types/miningProofTx';

type ReferrerOfResponse = {
  miner: string;
  referrer: string | null;
};

const POLL_INTERVAL = 15_000;

function useReferralInfo(address: string | undefined) {
  const queryClient = useCyberQueryClient();
  const [referralInfo, setReferralInfo] = useState<LithiumReferralInfoResponse | undefined>();
  const [loading, setLoading] = useState(false);
  const [counter, setCounter] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Store latest fetch function in a ref to avoid interval churn
  const fetchRef = useRef<() => void>(() => {});

  const fetchInfo = useCallback(async () => {
    if (!queryClient || !address) {
      setReferralInfo(undefined);
      return;
    }
    setLoading(true);
    try {
      const [infoData, referrerData] = await Promise.all([
        queryClient.queryContractSmart(
          LITIUM_REFER_CONTRACT,
          { referral_info: { address } }
        ).catch(() => null),
        queryClient.queryContractSmart(
          LITIUM_REFER_CONTRACT,
          { referrer_of: { miner: address } }
        ).catch(() => null),
      ]);

      const rawInfo = infoData as Omit<LithiumReferralInfoResponse, 'referrer'> | null;
      const referrerOf = referrerData as ReferrerOfResponse | null;

      if (rawInfo) {
        setReferralInfo({
          ...rawInfo,
          referrer: referrerOf?.referrer ?? null,
        });
      } else {
        setReferralInfo(undefined);
      }
    } catch {
      setReferralInfo(undefined);
    } finally {
      setLoading(false);
    }
  }, [queryClient, address]);

  fetchRef.current = fetchInfo;

  useEffect(() => {
    fetchInfo();
  }, [fetchInfo, counter]);

  // Periodic polling disabled — manual refetch via refetch() callback
  void intervalRef;

  const refetch = useCallback(() => {
    setCounter((c) => c + 1);
  }, []);

  return {
    referralInfo,
    loading,
    refetch,
  };
}

export default useReferralInfo;
