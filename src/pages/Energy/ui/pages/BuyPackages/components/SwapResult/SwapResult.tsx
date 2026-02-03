import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { AvailableAmount } from 'src/components';
import { useEnergy } from 'src/pages/Energy/context/Energy.context';
import {
  getExponentByDenom,
  getOsmoAssetByDenom,
} from 'src/pages/Energy/utils/utils';
import { useAppSelector } from 'src/redux/hooks';
import styles from './SwapResult.module.scss';

function SwapResult() {
  const { energyPackageSwapRoutes } = useEnergy();
  const { selectPlan, swapResult } = useAppSelector((state) => state.energy);

  const textInfo = swapResult ? 'swapped amount' : 'estimate amount';

  const selectPackage = energyPackageSwapRoutes?.find(
    (item) => item.keyPackage === selectPlan?.keyPackage
  );

  const coinMap = useMemo(() => {
    if (swapResult) {
      return swapResult.tokens;
    }

    if (selectPackage) {
      return selectPackage.swapInfo.map((item) => item.swap.tokenOut);
    }

    return [];
  }, [selectPackage, swapResult]);

  const renderItem = coinMap.map((item) => {
    const asset = getOsmoAssetByDenom(item.denom);
    const amount = new BigNumber(item.amount)
      .shiftedBy(-getExponentByDenom(asset?.base))
      .toString();
    return (
      <AvailableAmount
        key={item.denom}
        denom={asset?.symbol}
        amountToken={amount}
        title={textInfo}
      />
    );
  });

  return <div className={styles.wrapper}>{renderItem}</div>;
}

export default SwapResult;
