import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { DenomArr, FormatNumberTokens } from 'src/components';
import { DENOM_LIQUID } from 'src/constants/config';
import styles from './styles.module.scss';

type TitlePoolPool = {
  pool: Pool;
  totalCap: number;
  useInactive?: boolean;
};

function TitlePool({ pool, totalCap, useInactive }: TitlePoolPool) {
  return (
    <div className={styles.TitlePoolContainer}>
      <div className={styles.TitlePoolContainerInfoPool}>
        <div className={styles.TitlePoolContainerInfoPoolPoolText}>#{pool.id}</div>

        <div className={styles.TitlePoolContainerInfoPoolImg}>
          <DenomArr size={30} denomValue={pool.poolCoinDenom} onlyImg />
        </div>

        <div className={styles.TitlePoolContainerInfoPoolDenom}>
          <DenomArr denomValue={pool.poolCoinDenom} onlyText />
        </div>
      </div>
      {totalCap !== undefined && (
        <FormatNumberTokens value={totalCap} text={DENOM_LIQUID} marginContainer="0px" />
      )}
      {useInactive && <div className={styles.TitlePoolContainerInactive}>inactive</div>}
    </div>
  );
}

export default TitlePool;
