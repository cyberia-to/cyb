import { Coin } from '@cosmjs/launchpad';
import { useEffect, useMemo, useState } from 'react';
import { Display, DisplayTitle, FormatNumberTokens } from 'src/components';
import { useHub } from 'src/contexts/hub';
import { Option } from 'src/types';
import { ObjKeyValue } from 'src/types/data';
import { v4 as uuidv4 } from 'uuid';
import { exponentialToDecimal } from '../../../utils/utils';
import { PoolsWithAssetsCapType } from '../type';
import PoolItemsList from './pollItems';
import styles from './styles.module.scss';
import TitlePool from './TitlePoolCard';

type PoolCardProps = {
  pool: PoolsWithAssetsCapType;
  totalSupplyData: Option<ObjKeyValue>;
  accountBalances: Option<ObjKeyValue>;
  vol24: Option<Coin>;
};

function PoolCard({ pool, totalSupplyData, accountBalances, vol24 }: PoolCardProps) {
  const { tokens } = useHub();

  const [sharesToken, setSharesToken] = useState(null);

  useEffect(() => {
    setSharesToken(null);
    if (
      totalSupplyData &&
      Object.hasOwn(totalSupplyData, pool.poolCoinDenom) &&
      accountBalances &&
      Object.hasOwn(accountBalances, pool.poolCoinDenom)
    ) {
      const amountTotal = totalSupplyData[pool.poolCoinDenom];
      const amountAccountBalances = accountBalances[pool.poolCoinDenom];
      const procent = (amountAccountBalances / amountTotal) * 100;
      const shares = exponentialToDecimal(procent.toPrecision(2));
      setSharesToken(shares);
    }
  }, [totalSupplyData, pool, accountBalances]);

  const useInactive = useMemo(() => {
    try {
      let status = false;
      const { reserveCoinDenoms } = pool;
      if (reserveCoinDenoms && Object.keys(reserveCoinDenoms).length > 0) {
        reserveCoinDenoms.forEach((itemCoin) => {
          if (itemCoin.includes('ibc')) {
            status = !tokens?.[itemCoin];
          }
        });
      }

      return status;
    } catch (error) {
      console.log('error', error);
      return false;
    }
  }, [pool, tokens]);

  return (
    <Display
      title={
        <DisplayTitle
          title={<TitlePool useInactive={useInactive} pool={pool} totalCap={pool.cap} />}
        />
      }
    >
      <div>
        {pool.reserveCoinDenoms.map((items) => {
          const keyItem = uuidv4();

          return <PoolItemsList key={keyItem} assets={pool.assets} token={items} />;
        })}
      </div>

      {vol24 && (
        <div className={styles.PoolCardContainerMyShares}>
          <div className={styles.PoolCardContainerMySharesTitle}>Vol 24h</div>
          <div>
            <FormatNumberTokens value={vol24.amount} text={vol24.denom} />
          </div>
        </div>
      )}

      {sharesToken !== null && (
        <div className={styles.PoolCardContainerMyShares}>
          <div className={styles.PoolCardContainerMySharesTitle}>My share</div>
          <div>
            <FormatNumberTokens value={sharesToken} float customText="%" />
          </div>
        </div>
      )}
    </Display>
  );
}

export default PoolCard;
