import { Coin } from '@cosmjs/launchpad';
import { useQuery } from '@tanstack/react-query';
import axios from 'axios';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useState } from 'react';
import { DENOM_LIQUID, LCD_URL } from 'src/constants/config';
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

type OnChainSwapVolume = {
  pool_id: number;
  denom: string;
  amount: number;
}[];

const SWAP_TXS_LIMIT = 500;
const VOLUME_WINDOW_DAYS = 30;

const getWarpDexTickers = async (): Promise<Nullable<responseWarpDexTickersItem[]>> => {
  try {
    const response = await axios({
      method: 'get',
      url: 'https://warp-dex.cybernode.ai/dev/tickers/',
      timeout: 5000,
    });

    if (!Array.isArray(response.data)) {
      return null;
    }

    return response.data as responseWarpDexTickersItem[];
  } catch (_e) {
    return null;
  }
};

type OnChainSwapResult = {
  volumes: OnChainSwapVolume;
  actualDays: number;
};

const getOnChainSwapVolume = async (): Promise<Nullable<OnChainSwapResult>> => {
  try {
    const response = await axios.get(`${LCD_URL}/cosmos/tx/v1beta1/txs`, {
      params: {
        'pagination.limit': SWAP_TXS_LIMIT,
        order_by: 2,
        events: "message.action='/cyber.liquidity.v1beta1.MsgSwapWithinBatch'",
      },
      timeout: 15000,
    });

    const { tx_responses: txResponses } = response.data;
    if (!txResponses || !txResponses.length) {
      return null;
    }

    const now = new Date();
    const cutoff = new Date();
    cutoff.setDate(cutoff.getDate() - VOLUME_WINDOW_DAYS);

    const volumes: OnChainSwapVolume = [];
    let oldestTxTime = now;

    for (const tx of txResponses) {
      const txTime = new Date(tx.timestamp);
      if (txTime < cutoff) {
        break;
      }

      if (txTime < oldestTxTime) {
        oldestTxTime = txTime;
      }

      const messages = tx.tx?.body?.messages;
      if (!messages) {
        continue;
      }

      for (const msg of messages) {
        if (msg['@type'] === '/cyber.liquidity.v1beta1.MsgSwapWithinBatch') {
          const poolId = Number(msg.pool_id);
          const offerCoin = msg.offer_coin;
          if (offerCoin) {
            volumes.push({
              pool_id: poolId,
              denom: offerCoin.denom,
              amount: Number(offerCoin.amount),
            });
          }
        }
      }
    }

    if (volumes.length === 0) {
      return null;
    }

    const actualDays = Math.max(
      1,
      (now.getTime() - oldestTxTime.getTime()) / (1000 * 60 * 60 * 24)
    );

    return { volumes, actualDays };
  } catch (_e) {
    return null;
  }
};

export default function useWarpDexTickers() {
  const { marketData } = useAppData();
  const [vol24Total, setVol24Total] = useState<Coin | undefined>(undefined);
  const [vol24ByPool, setVol24ByPool] = useState<ObjectKey<Coin>>({}); // key is pool_id
  const { tracesDenom } = useIbcDenom();

  const { data: apiData } = useQuery({
    queryKey: ['warp-dex-tickers'],
    queryFn: async () => {
      const response = await getWarpDexTickers();
      return response ?? undefined;
    },
  });

  const { data: onChainData } = useQuery({
    queryKey: ['warp-onchain-volume'],
    queryFn: async () => {
      const response = await getOnChainSwapVolume();
      return response ?? undefined;
    },
    enabled: !apiData,
    staleTime: 5 * 60 * 1000,
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

  // Process API data
  useEffect(() => {
    if (!apiData || !Object.keys(marketData).length || !tracesDenom) {
      return;
    }

    let vol24Temp = new BigNumber(0);
    const listVol24ByPools: ObjectKey<Coin> = {};

    apiData.forEach((item: responseWarpDexTickersItem) => {
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
  }, [marketData, apiData, tracesDenom, getAmountVol]);

  // Process on-chain fallback data
  useEffect(() => {
    if (apiData || !onChainData || !Object.keys(marketData).length || !tracesDenom) {
      return;
    }

    const { volumes, actualDays } = onChainData;
    const poolVolumes: ObjectKey<BigNumber> = {};
    let totalVol = new BigNumber(0);

    for (const swap of volumes) {
      const vol = getAmountVol(swap.denom, swap.amount);
      if (vol.gt(0)) {
        const key = String(swap.pool_id);
        poolVolumes[key] = (poolVolumes[key] || new BigNumber(0)).plus(vol);
        totalVol = totalVol.plus(vol);
      }
    }

    // Average daily volume from actual time span
    const listVol24ByPools: ObjectKey<Coin> = {};

    Object.entries(poolVolumes).forEach(([poolId, vol]) => {
      const dailyAvg = vol.dividedBy(actualDays);
      listVol24ByPools[poolId] = {
        denom: DENOM_LIQUID,
        amount: dailyAvg.dp(0, BigNumber.ROUND_FLOOR).toString(10),
      };
    });

    const dailyTotal = totalVol.dividedBy(actualDays);

    setVol24ByPool(listVol24ByPools);
    setVol24Total({
      denom: DENOM_LIQUID,
      amount: dailyTotal.dp(0, BigNumber.ROUND_FLOOR).toString(10),
    });
  }, [marketData, apiData, onChainData, tracesDenom, getAmountVol]);

  return { data: apiData, vol24Total, vol24ByPool };
}
