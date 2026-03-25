import { DENOM_LIQUID } from 'src/constants/config';
import ItemBalance from '../components/ItemBalance/ItemBalance';
import styles from './LiquidBalances.module.scss';

type Props = {
  amount: {
    liquidH: number;
    frozenH: number;
  };
};

function LiquidBalances({ amount: { liquidH, frozenH } }: Props) {
  return (
    <div className={styles.wrapper}>
      <ItemBalance text="Liquid" amount={liquidH} demon={DENOM_LIQUID} />
      <ItemBalance text="Frozen" amount={frozenH} demon={DENOM_LIQUID} />
    </div>
  );
}

export default LiquidBalances;
