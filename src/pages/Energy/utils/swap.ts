import BigNumber from 'bignumber.js';
import { Asset } from '@chain-registry/types';
import { PrettyPair } from '@osmonauts/math/types';
import {
  calcAmountWithSlippage,
  getRoutesForTrade,
  noDecimals,
} from '@osmonauts/math';
import { osmosis } from 'osmojs';

import { Coin, coin } from '@cosmjs/stargate';
import { SwapAmountInRoute } from 'osmojs/osmosis/poolmanager/v1beta1/swap_route';
import { Pool } from 'osmojs/osmosis/gamm/v1beta1/balancerPool';
import { Osmosis } from './assets';
import { Swap, SwapTokensWithRoutes } from '../types/swap';
import { getExponentByDenom, isEmptyArray, newShiftedMinus } from './utils';
import { Token } from '../types/token';
import { assetsBuy } from './tokenBuy';
import { Slippages } from '../constants';

export function newToken(assets: Asset): Token {
  return {
    denom: assets.base,
    symbol: assets.symbol,
    amount: '',
  };
}

export function newCoin(token: Token): Coin {
  return {
    denom: token.denom,
    amount: new BigNumber(token.amount || '0')
      .shiftedBy(getExponentByDenom(token.denom))
      .toString(),
  };
}

function newRoutes(swap: Swap, pairs: PrettyPair[]) {
  if (!swap.from.denom || !swap.to.denom || !pairs.length) {
    return [];
  }

  const trade = {
    sell: newCoin(swap.from),
    buy: newCoin(swap.to),
  };

  try {
    return getRoutesForTrade(Osmosis.AssetsList, { trade, pairs });
  } catch (error) {
    console.log('error', error);
    return [];
  }
}

export function newSwapMessage(
  swapInfo: SwapTokensWithRoutes[],
  sender: string
) {
  const { swapExactAmountIn } =
    osmosis.poolmanager.v1beta1.MessageComposer.withTypeUrl;

  return swapInfo.map((item) => {
    const { routes, swap } = item;
    const { tokenIn, tokenOut } = swap;

    return swapExactAmountIn({
      sender,
      routes,
      tokenIn: coin(noDecimals(tokenIn.amount), tokenIn.denom),
      tokenOutMinAmount: noDecimals(tokenOut.amount),
    });
  });
}

function newPoolPrice(routes: SwapAmountInRoute[], pools: Pool[]) {
  const [{ poolId }] = routes;

  const pool = pools.find((item) => item.id === poolId);

  if (!pool) {
    return 0;
  }

  const { token: tokenA } = pool.poolAssets[0];
  const amountA = newShiftedMinus(tokenA).amount;

  const { token: tokenB } = pool.poolAssets[1];
  const amountB = newShiftedMinus(tokenB).amount;

  return new BigNumber(amountA).dividedBy(amountB).toNumber();
}

export function newTokensRoutes(
  amountValue: string,
  fromToken: Token,
  { pairs, pools }: { pairs: PrettyPair[]; pools: Pool[] }
) {
  if (isEmptyArray(pairs)) {
    return undefined;
  }
  const result: SwapTokensWithRoutes[] = [];

  const amountIn = new BigNumber(amountValue).dividedBy(3).toString();

  const tokenIn = newCoin({
    ...fromToken,
    amount: amountIn,
  });

  assetsBuy.forEach((item) => {
    const toToken = newToken(item);
    // find pool
    const routes = newRoutes(
      {
        from: fromToken,
        to: toToken,
      },
      pairs
    );

    if (!routes.length) {
      return;
    }

    // find pool price
    const poolPrice = newPoolPrice(routes, pools);
    const tokenOutMinAmount = calcAmountWithSlippage(
      new BigNumber(amountIn).multipliedBy(poolPrice || 1).toString(),
      Slippages[1]
    );

    const tokenOut = newCoin({
      ...toToken,
      amount: new BigNumber(tokenOutMinAmount)
        .dp(getExponentByDenom(toToken.denom), BigNumber.ROUND_FLOOR)
        .toString(),
    });

    const tokenOutConvert: Coin = {
      denom: item.display,
      amount: newShiftedMinus(tokenOut).amount,
    };

    result.push({
      swap: {
        tokenIn,
        tokenOut,
      },
      routes,
      tokenOutConvert,
    });
  });

  return result;
}
