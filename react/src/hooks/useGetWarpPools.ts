import { Coin } from '@cosmjs/launchpad';
import { useQuery } from '@tanstack/react-query';
import axios from 'axios';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useState } from 'react';
import { DENOM_LIQUID } from 'src/constants/config';
import { useAppData } from 'src/contexts/appData';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { Nullable } from 'src/types';
import { ObjectKey } from 'src/types/data';
import { getDisplayAmount } from 'src/utils/utils';

export type responseWarpDexTickersItem = {
  base_currency: string;
  target_currency: string;
  pool_id: number;
  ticker_id: string;
  last_price: number;
  liquidity_in_usd: number;
  base_volume: number;
  target_volume: number;
};

const getWarpDexTickers = async (): Promise<Nullable<responseWarpDexTickersItem[]>> => {
  try {
    const response = await axios({
      method: 'get',
      url: 'https://warp-dex.cybernode.ai/dev/tickers/',
    });

    return response.data as responseWarpDexTickersItem[];
  } catch (_e) {
    return null;
  }
};

export default function useWarpDexTickers() {
  const { marketData } = useAppData();
  const [vol24Total, setVol24Total] = useState<Coin | undefined>(undefined);
  const [vol24ByPool, setVol24ByPool] = useState<ObjectKey<Coin>>({}); // key is pool_id
  const { tracesDenom } = useIbcDenom();

  const { data } = useQuery({
    queryKey: ['warp-dex-tickers'],
    queryFn: async () => {
      const responce = await getWarpDexTickers();

      if (responce !== null) {
        return responce;
      }

      return undefined;
    },
  });

  const getAmountVol = useCallback(
    (denom: string, amount: number): BigNumber => {
      if (tracesDenom && Object.keys(marketData).length && Object.hasOwn(marketData, denom)) {
        const pollPrice = new BigNumber(marketData[denom]);
        const [{ coinDecimals }] = tracesDenom(denom);
        const reduceAmount = getDisplayAmount(amount, coinDecimals);
        const amountVol = pollPrice.multipliedBy(reduceAmount);

        return amountVol;
      }
      return new BigNumber(0);
    },
    [tracesDenom, marketData]
  );

  useEffect(() => {
    let vol24Temp = new BigNumber(0);
    const listVol24ByPools: ObjectKey<Coin> = {};

    if (Object.keys(marketData).length && data && tracesDenom) {
      data.forEach((item: responseWarpDexTickersItem) => {
        let vol24Item = new BigNumber(0);

        if (marketData[item.base_currency]) {
          const amount = getAmountVol(item.base_currency, item.base_volume);
          vol24Item = vol24Item.plus(amount);
        }

        if (marketData[item.target_currency]) {
          const amount = getAmountVol(item.target_currency, item.target_volume);
          vol24Item = vol24Item.plus(amount);
        }

        vol24Temp = vol24Temp.plus(vol24Item);

        listVol24ByPools[item.pool_id] = {
          denom: DENOM_LIQUID,
          amount: vol24Item.dp(0, BigNumber.ROUND_FLOOR).toString(10),
        };
      });

      setVol24ByPool(listVol24ByPools);
      setVol24Total({
        denom: DENOM_LIQUID,
        amount: vol24Temp.dp(0, BigNumber.ROUND_FLOOR).toString(10),
      });
    }
  }, [marketData, data, tracesDenom, getAmountVol]);

  return { data, vol24Total, vol24ByPool };
}
