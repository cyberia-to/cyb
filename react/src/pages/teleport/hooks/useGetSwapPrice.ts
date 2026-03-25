import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';

function useGetSwapPrice(
  tokenA: string,
  tokenB: string,
  tokenAPoolAmount: number,
  tokenBPoolAmount: number
): number {
  const [swapPrice, setSwapPrice] = useState<number>(0);

  useEffect(() => {
    let orderPrice = new BigNumber(0);
    setSwapPrice(0);

    const poolAmountA = new BigNumber(tokenAPoolAmount);
    const poolAmountB = new BigNumber(tokenBPoolAmount);

    if (poolAmountA.comparedTo(0) > 0 && poolAmountB.comparedTo(0) > 0) {
      if ([tokenA, tokenB].sort()[0] !== tokenA) {
        orderPrice = poolAmountB.dividedBy(poolAmountA);
        // orderPrice = orderPrice.multipliedBy(0.97);
      } else {
        orderPrice = poolAmountA.dividedBy(poolAmountB);
        // orderPrice = orderPrice.multipliedBy(1.03);
      }
    }

    setSwapPrice(orderPrice.toNumber());
  }, [tokenA, tokenB, tokenAPoolAmount, tokenBPoolAmount]);

  return swapPrice;
}

export default useGetSwapPrice;
