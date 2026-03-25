import { useQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

const keyQuery = 'validatorsBONDED';

function useGetValidatorsBonded() {
  const queryClient = useQueryClient();

  const [changeTimeAmount, setChangeTimeAmount] = useState({
    amount: 0,
    time: 0,
  });

  const { data, status } = useQuery({
    queryKey: [keyQuery],
    queryFn: async () => {
      let response = {
        validators: 0,
        timestamp: '',
      };

      const responseHeroesActive = await queryClient.validators('BOND_STATUS_BONDED');
      if (responseHeroesActive && responseHeroesActive !== null) {
        const { validators } = responseHeroesActive;
        const d = new Date();
        response = {
          validators: validators.length,
          timestamp: d,
        };
      }

      return response;
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
        const amountChange = new BigNumber(data.validators).minus(oldData.validators).toNumber();
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

  return { data, changeTimeAmount, status };
}

export default useGetValidatorsBonded;
