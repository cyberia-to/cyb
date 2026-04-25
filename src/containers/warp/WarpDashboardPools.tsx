/* eslint-disable no-restricted-syntax */

import { Coin } from '@cosmjs/launchpad';
import BigNumber from 'bignumber.js';
import { useMemo, useState } from 'react';
import { LinkWindow, MainContainer, NoItems } from 'src/components';
import Loader2 from 'src/components/ui/Loader2';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useGetBalances from 'src/hooks/getBalances';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useGetTotalSupply from 'src/hooks/useGetTotalSupply';
import useWarpDexTickers from 'src/hooks/useGetWarpPools';
import usePoolListInterval from 'src/hooks/usePoolListInterval';
import { v4 as uuidv4 } from 'uuid';
import useGetMySharesInPools from './hooks/useGetMySharesInPools';
import usePoolsAssetAmount from './hooks/usePoolsAssetAmount';
import { PoolCard, PoolsInfo } from './pool';
import styles from './pool/styles.module.scss';
import { calcPoolApr } from './utils';

type SortKey = 'tvl' | 'apr' | 'vol24';

function WarpDashboardPools() {
  const currentAddress = useCurrentAddress();
  const { liquidBalances: accountBalances } = useGetBalances(currentAddress);
  const { myCap } = useGetMySharesInPools(accountBalances);

  const { vol24Total, vol24ByPool } = useWarpDexTickers();
  const data = usePoolListInterval();
  const { poolsData, totalCap, loading } = usePoolsAssetAmount(data);
  const { totalSupplyAll } = useGetTotalSupply();

  useAdviserTexts({
    isLoading: loading,
    loadingText: 'loading pools',
    defaultText: (
      <span>
        warp is power dex for all things cyber <br />
        <LinkWindow to="https://api.warp-dex.cyb.ai/docs">api docs</LinkWindow>
      </span>
    ),
  });

  const useMyProcent = useMemo(() => {
    if (totalCap > 0 && myCap > 0) {
      return new BigNumber(myCap)
        .dividedBy(totalCap)
        .multipliedBy(100)
        .dp(3, BigNumber.ROUND_FLOOR)
        .toNumber();
    }

    return 0;
  }, [totalCap, myCap]);

  const [sortBy, setSortBy] = useState<SortKey>('tvl');

  const sortedPools = useMemo(() => {
    const pools = [...poolsData];

    pools.sort((a, b) => {
      if (sortBy === 'tvl') {
        return b.cap - a.cap;
      }

      const volA = vol24ByPool[a.id] ? parseFloat(vol24ByPool[a.id].amount) : 0;
      const volB = vol24ByPool[b.id] ? parseFloat(vol24ByPool[b.id].amount) : 0;

      if (sortBy === 'vol24') {
        return volB - volA;
      }

      // apr
      const aprA = calcPoolApr(volA, a.cap);
      const aprB = calcPoolApr(volB, b.cap);
      return aprB - aprA;
    });

    return pools;
  }, [poolsData, vol24ByPool, sortBy]);

  const itemsPools = useMemo(() => {
    return (
      sortedPools.map((item) => {
        const keyItem = uuidv4();
        let vol24: Coin | undefined;

        if (Object.hasOwn(vol24ByPool, item.id)) {
          vol24 = vol24ByPool[item.id];
        }

        return (
          <PoolCard
            key={keyItem}
            pool={item}
            totalSupplyData={totalSupplyAll}
            accountBalances={accountBalances}
            vol24={vol24}
          />
        );
      }) || []
    );
  }, [sortedPools, vol24ByPool, accountBalances, totalSupplyAll]);

  return (
    <MainContainer>
      <div className={styles.PoolDataContainer}>
        <PoolsInfo
          myCap={myCap}
          totalCap={totalCap}
          useMyProcent={useMyProcent}
          vol24={vol24Total}
        />
        <div className={styles.sortBar}>
          {(['tvl', 'apr', 'vol24'] as SortKey[]).map((key) => (
            <button
              key={key}
              type="button"
              className={sortBy === key ? styles.sortBtnActive : styles.sortBtn}
              onClick={() => setSortBy(key)}
            >
              {key === 'tvl' ? 'TVL' : key === 'apr' ? 'APR' : 'Vol 24h'}
            </button>
          ))}
        </div>

        {loading ? (
          <Loader2 />
        ) : Object.keys(itemsPools).length > 0 ? (
          <div className={styles.pools}>{itemsPools}</div>
        ) : (
          <NoItems text="No Pools" />
        )}
      </div>
    </MainContainer>
  );
}

export default WarpDashboardPools;
