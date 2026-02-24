import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useAccountCountQuery } from 'src/generated/graphql';

function useAccountCount() {
  const keyQuery = 'accountCount';
  const [changeTimeAmount, setChangeTimeAmount] = useState({
    amount: 0,
    time: 0,
  });
  const { data } = useAccountCountQuery({ pollInterval: 1000 * 60 * 3 });

  useEffect(() => {
    if (data) {
      const timestamp = new Date();
      const lastgraphStatsLs = localStorage.getItem(keyQuery);
      if (lastgraphStatsLs !== null) {
        const oldData = JSON.parse(lastgraphStatsLs);
        const timeChange = Date.parse(timestamp) - Date.parse(oldData.timestamp);
        const amountChange = new BigNumber(data.account_aggregate.aggregate?.count || 0)
          .minus(oldData.accountCount)
          .toNumber();
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

  return {
    data: {
      accountCount: data?.account_aggregate.aggregate?.count || 0,
    },
    changeTimeAmount,
  };
}

export default useAccountCount;
