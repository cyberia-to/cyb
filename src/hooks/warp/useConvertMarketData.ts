import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { DENOM_LIQUID } from 'src/constants/config';
import { ObjKeyValue } from 'src/types/data';
import useFindPoolPrice from './useFindPoolPrice';

const LIQUID_PRICE = 1;

function useConvertMarketData(marketData: ObjKeyValue) {
  const poolPrice = useFindPoolPrice();

  const resultData = useMemo(() => {
    if (!poolPrice || !Object.keys(marketData).length) {
      return {};
    }

    return Object.entries(marketData).reduce(
      (acc, [key, value]) => ({
        ...acc,
        [key]:
          key === DENOM_LIQUID
            ? LIQUID_PRICE
            : new BigNumber(value).multipliedBy(poolPrice).toNumber(),
      }),
      {}
    );
  }, [poolPrice, marketData]);

  return resultData;
}

export default useConvertMarketData;
