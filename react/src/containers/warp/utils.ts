import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import BigNumber from 'bignumber.js';
import { ObjKeyValue } from 'src/types/data';
import { MyPoolsT } from './type';

const reduceTextCoin = (text: string) => {
  switch (text) {
    case 'millivolt':
      return 'V';

    case 'milliampere':
      return 'A';

    case 'hydrogen':
      return 'H';

    case 'boot':
      return 'BOOT';

    default:
      return text;
  }
};

export function getPoolToken(pool: Pool[], myPoolTokens: ObjKeyValue): MyPoolsT[] {
  const myPools: MyPoolsT[] = [];

  pool.forEach((item) => {
    if (myPoolTokens[item.poolCoinDenom]) {
      const myTokenAmount = myPoolTokens[item.poolCoinDenom];
      myPools.push({
        ...item,
        coinDenom: `${reduceTextCoin(
          item.reserveCoinDenoms[0]
        )}-${reduceTextCoin(item.reserveCoinDenoms[1])}`,
        myTokenAmount,
      });
    }
  });

  return myPools;
}

function getMyTokenBalance(token, indexer) {
  if (indexer === null) {
    return 0;
  }
  const balance = Number(Number(indexer[token])).toFixed(2);
  if (balance !== 'NaN') {
    return balance;
  }
  return 0;
}

export function getMyTokenBalanceNumber(denom: string, indexer) {
  return Number(getMyTokenBalance(denom, indexer));
}

function pow(coinDecimals) {
  return new BigNumber(1).multipliedBy(new BigNumber(10).pow(coinDecimals));
}

export function calculateCounterPairAmount(values, state) {
  const inputAmount = values;

  let counterPairAmount = new BigNumber(0);
  let swapPrice = new BigNumber(0);

  const {
    tokenAPoolAmount,
    tokenBPoolAmount,
    tokenA,
    tokenB,
    tokenACoinDecimals,
    tokenBCoinDecimals,
    isReverse,
  } = state;

  const poolAmountA = new BigNumber(tokenAPoolAmount);
  const poolAmountB = new BigNumber(tokenBPoolAmount);

  const amount = new BigNumber(inputAmount).multipliedBy(
    pow(isReverse ? tokenBCoinDecimals : tokenACoinDecimals)
  );

  const isPoolPair = [tokenA, tokenB].sort()[0] === tokenA;

  let poolCoins: BigNumber[] = [];

  if (isPoolPair) {
    poolCoins = [poolAmountA, poolAmountB];
  } else {
    poolCoins = [poolAmountB, poolAmountA];
  }

  if ((isPoolPair && !isReverse) || (!isPoolPair && isReverse)) {
    swapPrice = poolCoins[1].dividedBy(poolCoins[0]);
  }

  if ((isPoolPair && isReverse) || (!isPoolPair && !isReverse)) {
    swapPrice = poolCoins[0].dividedBy(poolCoins[1]);
  }

  if (isReverse) {
    counterPairAmount = amount
      .multipliedBy(swapPrice.multipliedBy(new BigNumber(1)))
      .dividedBy(pow(tokenACoinDecimals))
      .dp(tokenACoinDecimals, BigNumber.ROUND_FLOOR);
  } else {
    counterPairAmount = amount
      .multipliedBy(swapPrice)
      .dividedBy(pow(tokenBCoinDecimals))
      .dp(tokenBCoinDecimals, BigNumber.ROUND_FLOOR);
  }

  return {
    counterPairAmount,
  };
}
