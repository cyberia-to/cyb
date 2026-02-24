import { useQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { getConfigGift, getNumTokens, getStateGift } from '../../portal/utils';

function useGetPortalStats() {
  const queryClient = useQueryClient();
  const keyQuery = 'portalStats';
  const [changeTimeAmount, setChangeTimeAmount] = useState({
    citizensAmount: 0,
    procentClaimAmount: 0,
    time: 0,
  });

  const { data, status } = useQuery({
    queryKey: [keyQuery],
    queryFn: async () => {
      let response = { citizens: 0, procentClaim: 0, timestamp: '' };

      const queryResponseResultState = await getStateGift(queryClient);
      const queryResponseConfigGift = await getConfigGift(queryClient);
      const respnseNumTokens = await getNumTokens(queryClient);

      if (queryResponseConfigGift !== null && queryResponseResultState !== null) {
        const { current_balance: currentBalance } = queryResponseResultState;
        const { initial_balance: initialBalance } = queryResponseConfigGift;
        const claimAmount = new BigNumber(currentBalance).dividedBy(initialBalance);
        const procentClaim = new BigNumber(1)
          .minus(claimAmount)
          .multipliedBy(100)
          .dp(3, BigNumber.ROUND_FLOOR)
          .toNumber();
        response = { ...response, procentClaim };
      }
      if (respnseNumTokens?.count) {
        const countCitizens = parseFloat(respnseNumTokens.count);
        response = { ...response, citizens: countCitizens };
      }
      if (Object.keys(response).length > 0) {
        const d = new Date();
        response = { ...response, timestamp: d };
        return response;
      }
      return null;
    },
    refetchInterval: 1000 * 60 * 3,
    enabled: Boolean(queryClient),
  });

  useEffect(() => {
    if (data && data !== null) {
      const lastgraphStatsLs = localStorage.getItem(keyQuery);
      if (lastgraphStatsLs !== null) {
        const oldData = JSON.parse(lastgraphStatsLs);

        const timeChange = Date.parse(data.timestamp) - Date.parse(oldData.timestamp);
        const citizensAmount = new BigNumber(data.citizens).minus(oldData.citizens).toNumber();
        const procentClaimAmount = new BigNumber(data.procentClaim)
          .minus(oldData.procentClaim)
          .toNumber();

        if (citizensAmount > 0) {
          setChangeTimeAmount((item) => ({ ...item, citizensAmount }));
        }
        if (procentClaimAmount > 0) {
          setChangeTimeAmount((item) => ({ ...item, procentClaimAmount }));
        }
        const validChange = citizensAmount > 0 || citizensAmount > 0;
        if (timeChange > 0 && validChange) {
          setChangeTimeAmount((item) => ({ ...item, time: timeChange }));
        }
      }
      localStorage.setItem(keyQuery, JSON.stringify(data));
    }
  }, [data]);

  return { data, changeTimeAmount, status };
}

export default useGetPortalStats;
