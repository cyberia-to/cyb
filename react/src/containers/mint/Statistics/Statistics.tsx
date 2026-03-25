import { CardStatisics, ValueImg } from 'src/components';
import { formatNumber } from 'src/utils/utils';
import styles from './Statistics.module.scss';

function Statistics({ amount }: { amount: { vestedA: number; vestedV: number } }) {
  const { vestedA, vestedV } = amount;

  return (
    <div className={styles.container}>
      <CardStatisics title={<ValueImg text="milliampere" />} value={formatNumber(vestedA)} />
      <CardStatisics title={<ValueImg text="millivolt" />} value={formatNumber(vestedV)} />
      <CardStatisics title="My Energy" value={`${formatNumber(vestedA * vestedV)} W`} />
    </div>
  );
}

export default Statistics;
