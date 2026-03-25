import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useAppData } from 'src/contexts/appData';

const useGetMySharesInPools = (accountBalances) => {
  const [myCap, setMyCap] = useState(0);
  const { marketData } = useAppData();

  useEffect(() => {
    let myCaptemp = new BigNumber(0);
    if (accountBalances !== null) {
      const filtered = Object.keys(accountBalances)
        .filter((key) => key.includes('pool'))
        .reduce((obj, key) => {
          return { ...obj, [key]: accountBalances[key] };
        }, {});
      if (Object.keys(filtered).length > 0) {
        Object.keys(filtered).forEach((key) => {
          const amount = new BigNumber(filtered[key]);
          if (Object.hasOwn(marketData, key)) {
            const price = new BigNumber(marketData[key]);
            myCaptemp = myCaptemp.plus(amount.multipliedBy(price));
          }
        });
      }
      setMyCap(myCaptemp.dp(0, BigNumber.ROUND_FLOOR).toNumber());
    }
  }, [accountBalances, marketData]);

  return { myCap };
};

export default useGetMySharesInPools;
