import { formatNumber } from 'src/utils/utils';
import styles from './AvailableAmount.module.scss';
import LinearGradientContainer, {
  Color,
} from '../LinearGradientContainer/LinearGradientContainer';
import DenomArr from '../denom/denomArr';

type Props = {
  amountToken: number | string;
  title?: string;
  denom?: string;
  color?: Color;
};

function AvailableAmount({
  amountToken,
  title = 'available amount',
  denom,
  color = Color.Black,
}: Props) {
  return (
    <div className={styles.containerAvailableAmount}>
      <LinearGradientContainer color={color} title={title}>
        <div
          className={styles.containerValue}
          style={{ gap: denom ? '5px' : 0 }}
        >
          {formatNumber(amountToken)}
          {denom && (
            <DenomArr denomValue={denom} onlyImg tooltipStatusImg={false} />
          )}
        </div>
      </LinearGradientContainer>
    </div>
  );
}

export default AvailableAmount;
