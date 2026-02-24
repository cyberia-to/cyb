import { useQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

const keyQuery = 'negentropy';

function useGetNegentropy(refetchInterval: number | undefined) {
  const queryClient = useQueryClient();
  const [changeTimeAmount, setChangeTimeAmount] = useState({
    amount: 0,
    time: 0,
  });

  const { data, status } = useQuery({
    queryKey: [keyQuery],
    queryFn: async () => {
      const result = await queryClient?.negentropy();
      return { negentropy: result?.negentropy || 0, timestamp: Date.now() };
    },
    enabled: Boolean(queryClient),
    refetchInterval,
  });

  useEffect(() => {
    if (data) {
      const lastgraphStatsLs = localStorage.getItem(keyQuery);
      if (lastgraphStatsLs !== null) {
        const oldData = JSON.parse(lastgraphStatsLs);
        const timeChange = data!.timestamp - oldData.timestamp;
        const amountChange = new BigNumber(data!.negentropy).minus(oldData.negentropy).toNumber();
        if (timeChange > 0 && amountChange > 0) {
          setChangeTimeAmount({
            time: timeChange,
            amount: amountChange,
          });
        }
      }
      localStorage.setItem(keyQuery, JSON.stringify(data));
    }
  }, [data]);

  return { data, status, changeTimeAmount };
}

export default useGetNegentropy;
