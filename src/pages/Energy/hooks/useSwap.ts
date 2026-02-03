import { useMemo } from 'react';
import BigNumber from 'bignumber.js';
import { useAppSelector } from 'src/redux/hooks';
import { newCoin, newToken, newTokensRoutes } from '../utils/swap';
import {
  getOsmoAssetByDenom,
  isEmptyArray,
  symbolToOsmoDenom,
} from '../utils/utils';
import { EnergyPackages } from '../constants';
import usePrices from './usePrices';
import usePools from './usePools';
import { makePoolPairs } from '../utils/pool';
import { EnergyPackageSwapRoutes } from '../types/EnergyPackages';

function useSwap() {
  const { freefloat: pools, refetch: refetchPools } = usePools();
  const { data: prices, refetch: refetchPrice } = usePrices();
  const { tokenSell } = useAppSelector((state) => state.energy);

  const findTokenDenom = getOsmoAssetByDenom(
    symbolToOsmoDenom(tokenSell) || ''
  );

  const fromToken = newToken(findTokenDenom);

  const pricesSelectedDenom = prices[fromToken.denom] || 0;

  const energyPackagesByDenom: { [key: string]: string } =
    EnergyPackages.reduce(
      (acc, item) => ({
        ...acc,
        [item]: new BigNumber(item)
          .multipliedBy(pricesSelectedDenom)
          .toString(),
      }),
      {}
    );

  const pairs = useMemo(() => {
    if (isEmptyArray(pools)) {
      return [];
    }

    return makePoolPairs(pools, prices);
  }, [pools, prices]);

  const energyPackageSwapRoutes = useMemo(() => {
    if (!pricesSelectedDenom || isEmptyArray(pairs) || isEmptyArray(pools)) {
      return undefined;
    }

    const result: EnergyPackageSwapRoutes[] = [];

    Object.keys(energyPackagesByDenom).forEach((key) => {
      const item = energyPackagesByDenom[key];
      const swapRoute = newTokensRoutes(item, fromToken, { pairs, pools });
      if (swapRoute) {
        const tokenOut = swapRoute.map((item) => item.tokenOutConvert);

        result.push({
          keyPackage: key,
          tokenIn: newCoin({ denom: fromToken.denom, amount: item }),
          swapInfo: swapRoute,
          tokenOut,
        });
      }
    });

    return result;
  }, [pricesSelectedDenom, pairs, pools, energyPackagesByDenom, fromToken]);

  const refetchSwapRoute = () => {
    refetchPools();
    refetchPrice();
  };

  console.log('energyPackageSwapRoutes', energyPackageSwapRoutes);

  return { energyPackageSwapRoutes, prices, refetchSwapRoute };
}

export default useSwap;
