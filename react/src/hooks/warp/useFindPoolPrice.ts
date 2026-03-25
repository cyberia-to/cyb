import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import defaultNetworks from 'src/constants/defaultNetworks';
import { Networks } from 'src/types/networks';
import { getDenomHash, reduceBalances } from 'src/utils/utils';
import useConnectBostrom from './api/useConnectBostrom';
import useGetPoolsWarp from './api/useGetPoolsWarp';

const BOSTROM_CONFIG = defaultNetworks[Networks.BOSTROM];
const PUSSY_CONFIG = defaultNetworks[Networks.SPACE_PUSSY];

function useFindPoolPrice() {
  const [poolPrice, setPoolPrice] = useState<number | undefined>(undefined);
  const { queryClient } = useConnectBostrom();
  const { data } = useGetPoolsWarp(queryClient);

  const denomIbcLP = getDenomHash('transfer/channel-11', PUSSY_CONFIG.DENOM_LIQUID);

  useEffect(() => {
    (async () => {
      if (!data || !queryClient) {
        return;
      }

      const findPool = data.pools.find(
        (item) =>
          item.reserveCoinDenoms[0] === BOSTROM_CONFIG.DENOM_LIQUID &&
          item.reserveCoinDenoms[1] === denomIbcLP
      );

      if (!findPool) {
        return;
      }

      const tokenA = findPool.reserveCoinDenoms[0];
      const tokenB = findPool.reserveCoinDenoms[1];

      const balances = await queryClient.getAllBalances(findPool.reserveAccountAddress);

      const amount = reduceBalances(balances);

      const price = new BigNumber(amount[tokenA]).div(amount[tokenB]).toNumber();

      setPoolPrice(price);
    })();
  }, [data, queryClient, denomIbcLP]);

  return poolPrice;
}

export default useFindPoolPrice;
