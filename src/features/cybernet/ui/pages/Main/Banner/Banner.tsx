import { AmountDenom } from 'src/components';
import { TypingText } from 'src/containers/temple/pages/play/PlayBanerContent';
import { useCybernet } from '../../../cybernet.context';
import styles from './Banner.module.scss';

function Banner() {
  const { contracts } = useCybernet();

  const totalPaid = contracts.reduce(
    (acc, contract) => acc + Number(contract.economy?.total_rewards?.amount || 0),
    0
  );

  return (
    <div className={styles.banner}>
      <h2>
        <TypingText content="teaching and learning must be rewarded" delay={30} />
      </h2>

      <div className={styles.center}>
        <img src={require('./logo.png')} alt="cyberver" />

        {!!totalPaid && (
          <div className={styles.rewardsBlock}>
            <AmountDenom amountValue={totalPaid} denom="pussy" />

            <span className={styles.rewardsText}>rewards payed</span>
          </div>
        )}
      </div>

      <h1>
        <TypingText content="cyberver" />
      </h1>
    </div>
  );
}

export default Banner;
