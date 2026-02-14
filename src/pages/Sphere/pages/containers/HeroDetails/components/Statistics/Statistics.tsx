import { CardStatisics } from 'src/components';
import { BASE_DENOM } from 'src/constants/config';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import { Coin } from '@cosmjs/stargate';
import { DecCoin } from 'cosmjs-types/cosmos/base/v1beta1/coin';
import KeybaseAvatar from 'src/pages/Sphere/pages/components/KeybaseAvatar/keybaseAvatar';
import { Decimal } from '@cosmjs/math';
import { StatusTooltip } from '../../../Heroes/components/ui';
import styles from './Statistics.module.scss';

type Props = {
  data: {
    staked?: Coin;
    reward?: DecCoin;
    info: { moniker: string; identity: string; status: number };
  };
};

function Statistics({ data }: Props) {
  const { staked, reward, info } = data;

  return (
    <div className={styles.wrapper}>
      <CardStatisics
        title="staked"
        value={
          <IconsNumber
            value={staked?.amount || 0}
            type={staked?.denom || BASE_DENOM}
          />
        }
      />

      <div className={styles.identityContainer}>
        <KeybaseAvatar identity={info.identity} />
        <div className={styles.monikerContainer}>
          <StatusTooltip status={info.status} size={10} animation />
          <span>{info.moniker}</span>
        </div>
      </div>

      <CardStatisics
        title="rewards"
        value={
          <IconsNumber
            value={
              reward?.amount
                ? Decimal.fromAtomics(reward.amount, 18).floor().toString()
                : 0
            }
            type={reward?.denom || BASE_DENOM}
          />
        }
      />
    </div>
  );
}

export default Statistics;
