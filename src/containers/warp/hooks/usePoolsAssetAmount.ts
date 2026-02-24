import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { useQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useAppData } from 'src/contexts/appData';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useQueryClient } from 'src/contexts/queryClient';
import { Option } from 'src/types';
import { convertAmount, reduceBalances } from '../../../utils/utils';
import { AssetsType, OptionNeverArray, PoolsWithAssetsCapType, PoolsWithAssetsType } from '../type';

const usePoolsAssetAmount = (pools: Option<Pool[]>) => {
  const queryClient = useQueryClient();
  const { marketData } = useAppData();
  const { tracesDenom } = useIbcDenom();

  const [poolsData, setPoolsData] = useState<OptionNeverArray<PoolsWithAssetsCapType[]>>([]);
  const [totalCap, setTotalCap] = useState<number>(0);

  useEffect(() => {
    const lastPoolCapLocalStorage = localStorage.getItem('lastPoolCap');
    // const lastPoolLocalStorage = localStorage.getItem('lastPoolData');

    if (lastPoolCapLocalStorage !== null) {
      setTotalCap(new BigNumber(lastPoolCapLocalStorage).toNumber());
    }

    // if (lastPoolLocalStorage !== null) {
    //   const lastPoolLSData = JSON.parse(lastPoolLocalStorage);
    //   if (lastPoolLSData.length > 0) {
    //     setPoolsData(lastPoolLSData);
    //   }
    // }
  }, []);

  const {
    data: poolsBal,
    isLoading,
    isInitialLoading,
    isFetching,
    error,
  } = useQuery(
    ['getAllBalances', pools?.map((pool) => pool.id).join(',')],
    async () => {
      const newArrPools: PoolsWithAssetsType[] = [];
      // eslint-disable-next-line no-restricted-syntax
      for await (const pool of pools!) {
        const assetsData: AssetsType = {};
        const { reserveAccountAddress } = pool;

        const getBalancePromise = await queryClient!.getAllBalances(reserveAccountAddress);

        const dataReduceBalances = reduceBalances(getBalancePromise);
        Object.keys(dataReduceBalances).forEach((key) => {
          const amount = new BigNumber(dataReduceBalances[key]).toNumber();
          const [{ coinDecimals }] = tracesDenom(key);
          const reduceAmoun = convertAmount(amount, coinDecimals);
          assetsData[key] = reduceAmoun;
        });
        if (Object.keys(assetsData).length > 0) {
          newArrPools.push({ ...pool, assets: { ...assetsData } });
        }
      }
      return newArrPools;
    },
    {
      enabled: Boolean(queryClient && pools?.length > 0),
    }
  );

  // TODO: replace useEffect with useMemo
  useEffect(() => {
    if (poolsBal?.length > 0) {
      const newArrPools: PoolsWithAssetsCapType[] = [];
      let totalCapTemp = new BigNumber(0);
      poolsBal.forEach((pool) => {
        const { reserveCoinDenoms, assets } = pool;
        let cap = new BigNumber(0);
        reserveCoinDenoms.forEach((item) => {
          if (
            Object.keys(marketData).length > 0 &&
            Object.hasOwn(assets, item) &&
            Object.hasOwn(marketData, item)
          ) {
            const amountA = new BigNumber(assets[item]);
            const priceA = marketData[item];
            const capItem = amountA.multipliedBy(priceA);
            cap = cap.plus(capItem);
          }
        });
        totalCapTemp = totalCapTemp.plus(cap);
        newArrPools.push({
          ...pool,
          cap: cap.dp(0, BigNumber.ROUND_FLOOR).toNumber(),
        });
      });

      setTotalCap(totalCapTemp.dp(0, BigNumber.ROUND_FLOOR).toNumber());
      if (totalCapTemp.comparedTo(0)) {
        localStorage.setItem('lastPoolCap', totalCapTemp.dp(0, BigNumber.ROUND_FLOOR).toString());
      }

      if (Object.keys(marketData).length > 0) {
        const sortedArr = newArrPools.sort((a, b) => b.cap - a.cap);
        setPoolsData(sortedArr);
        localStorage.setItem('lastPoolData', JSON.stringify(sortedArr));
      } else {
        setPoolsData(newArrPools);
        localStorage.setItem('lastPoolData', JSON.stringify(newArrPools));
      }
    }
  }, [poolsBal, marketData]);

  return { poolsData, totalCap, loading: isLoading };
};

export default usePoolsAssetAmount;
