import { FormatNumberTokens } from 'src/components';
import styles from './ItemBalance.module.scss';

type Props = {
  text: string;
  amount: number;
  demon: string;
};

function ItemBalance({ text, amount, demon }: Props) {
  return (
    <div className={styles.wrapper}>
      <span className={styles.text}>{text}</span>
      <FormatNumberTokens value={amount} text={demon} styleValue={{ fontSize: '16px' }} />
    </div>
  );
}

export default ItemBalance;
