import { formatNumber } from 'src/utils/utils';
import LinearGradientContainer, { Color } from '../LinearGradientContainer/LinearGradientContainer';
import styles from './AvailableAmount.module.scss';

type Props = {
  amountToken: number | string;
  title?: string;
};

function AvailableAmount({ amountToken, title = 'available amount' }: Props) {
  return (
    <div className={styles.containerAvailableAmount}>
      <LinearGradientContainer color={Color.Black} title={title}>
        <div className={styles.containerValue}>{formatNumber(amountToken)}</div>
      </LinearGradientContainer>
    </div>
  );
}

export default AvailableAmount;
