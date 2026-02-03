import { useEnergy } from 'src/pages/Energy/context/Energy.context';
import { symbolToOsmoDenom } from 'src/pages/Energy/utils/utils';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { useEffect, useMemo } from 'react';
import { StatusOrder } from 'src/pages/Energy/redux/utils';
import { setStatusOrder } from 'src/pages/Energy/redux/energy.redux';
import { DenomArr, Select } from 'src/components';
import defaultNetworks from 'src/constants/defaultNetworks';
import ActionBarContainer from './ActionBar';
import Progress from './components/Progress/Progress';
import Header from './components/Header/Header';
import SwapResult from './components/SwapResult/SwapResult';
import BalancesInfo from './components/BalancesInfo/BalancesInfo';
import styles from './BuyPackages.module.scss';
import PackageSelected from './components/Header/ui/PackageSelected/PackageSelected';

function BuyPackages() {
  const { balances } = useEnergy();
  const { selectPlan, tokenSell, statusOrder } = useAppSelector(
    (state) => state.energy
  );
  const dispatch = useAppDispatch();

  const balancesTokenSell = useMemo(() => {
    const denom = symbolToOsmoDenom(tokenSell);
    if (!denom) {
      return undefined;
    }
    return balances.hash[denom];
  }, [balances.hash, tokenSell]);

  useEffect(() => {
    if (statusOrder !== StatusOrder.SELECT_PACK || !selectPlan) {
      return;
    }

    if (
      balancesTokenSell &&
      parseFloat(balancesTokenSell.amount) > 0 &&
      parseFloat(balancesTokenSell.amount) >
        parseFloat(selectPlan.tokenIn.amount)
    ) {
      setTimeout(() => {
        dispatch(setStatusOrder(StatusOrder.SWAP));
      }, 1000);
    }
  }, [balancesTokenSell, dispatch, selectPlan, statusOrder]);

  return (
    <>
      <PackageSelected />

      <Header />

      <Progress status={statusOrder} />

      <div className={styles.containerContentInfo}>
        <BalancesInfo balancesTokenSell={balancesTokenSell} />
        <SwapResult />
        <Select
          valueSelect="bostrom"
          currentValue="bostrom"
          disabled
          width="160px"
          options={[
            {
              value: 'bostrom',
              text: <span>bostrom</span>,
              img: (
                <DenomArr
                  denomValue={defaultNetworks.bostrom.CHAIN_ID}
                  onlyImg
                  type="network"
                  tooltipStatusImg={false}
                />
              ),
            },
          ]}
          title="send to"
        />
      </div>

      <ActionBarContainer />
    </>
  );
}

export default BuyPackages;
