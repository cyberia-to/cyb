import { Coin } from '@cosmjs/launchpad';
import { ContainerGradientText, FormatNumberTokens } from 'src/components';
import { DENOM_LIQUID } from 'src/constants/config';
import styles from './styles.module.scss';

type Props = {
  vol24: undefined | Coin;
  totalCap: number;
  myCap: number;
  useMyProcent: number;
};

function PoolsInfo({ totalCap, myCap, useMyProcent, vol24 }: Props) {
  return (
    <div className={styles.PoolsInfoContainer}>
      <ContainerGradientText>
        <div className={styles.PoolsInfoContainerValue}>
          <FormatNumberTokens styleValue={{ fontSize: '18px' }} text={DENOM_LIQUID} value={myCap} />
        </div>

        <div className={styles.PoolsInfoContainerText}>My liquidity</div>
      </ContainerGradientText>
      <ContainerGradientText>
        <div className={styles.PoolsInfoContainerValue}>
          <FormatNumberTokens
            value={totalCap}
            styleValue={{ fontSize: '18px' }}
            text={DENOM_LIQUID}
          />
        </div>

        <div className={styles.PoolsInfoContainerText}>Liquidity</div>
      </ContainerGradientText>
      <ContainerGradientText>
        <div className={styles.PoolsInfoContainerValue}>{useMyProcent}%</div>
        <div className={styles.PoolsInfoContainerText}>My share</div>
      </ContainerGradientText>
      <ContainerGradientText>
        <div className={styles.PoolsInfoContainerValue}>
          <FormatNumberTokens
            value={vol24?.amount ? parseFloat(vol24.amount) : 0}
            styleValue={{ fontSize: '18px' }}
            text={DENOM_LIQUID}
          />
        </div>
        <div className={styles.PoolsInfoContainerText}>Volume 24h</div>
      </ContainerGradientText>
    </div>
  );
}

export default PoolsInfo;
