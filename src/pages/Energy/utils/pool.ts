import { PrettyPair, PriceHash } from '@osmonauts/math/types';
import BigNumber from 'bignumber.js';
import { Pool } from 'osmojs/osmosis/gamm/v1beta1/balancerPool';
import { getOsmoAssetByDenom, newShiftedMinus } from './utils';

const calcPoolLiquidity = (pool: Pool, prices: PriceHash): string => {
  const { token: tokenA } = pool.poolAssets[0];
  const amountA = new BigNumber(newShiftedMinus(tokenA).amount).multipliedBy(
    prices[tokenA.denom] || 0
  );

  const { token: tokenB } = pool.poolAssets[1];
  const amountB = new BigNumber(newShiftedMinus(tokenB).amount).multipliedBy(
    prices[tokenB.denom] || 0
  );

  return amountA.plus(amountB).toString();
};

// eslint-disable-next-line import/prefer-default-export
export const makePoolPairs = (
  pools: Pool[],
  prices: PriceHash,
  liquidityLimit = 200
): PrettyPair[] => {
  return pools
    .filter(
      (pool) =>
        pool.poolAssets.length === 2 &&
        pool.poolAssets.every(({ token }) => !token.denom.startsWith('gamm')) &&
        new BigNumber(calcPoolLiquidity(pool, prices)).gte(liquidityLimit)
    )
    .map((pool) => {
      const assetA = pool.poolAssets[0].token;
      const assetAinfo = getOsmoAssetByDenom(assetA.denom);
      const assetB = pool.poolAssets[1].token;
      const assetBinfo = getOsmoAssetByDenom(assetB.denom);

      if (!assetAinfo || !assetBinfo) {
        return;
      }

      return {
        poolId: BigInt(pool.id).toString(),
        poolAddress: pool.address,
        baseName: assetAinfo.display,
        baseSymbol: assetAinfo.symbol,
        baseAddress: assetAinfo.base,
        quoteName: assetBinfo.display,
        quoteSymbol: assetBinfo.symbol,
        quoteAddress: assetBinfo.base,
      };
    })
    .filter(Boolean);
};
