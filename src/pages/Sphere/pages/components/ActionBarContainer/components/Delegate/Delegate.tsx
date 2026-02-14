import { InputNumber } from 'src/components';
import styles from './Delegate.module.scss';

type Props = {
  delegate: boolean;
  maxValue?: number;
  value: string | number;
  moniker?: string;
  onChangeInputAmount: (val: string) => void;
};

function Delegate({
  moniker,
  delegate,
  maxValue,
  value,
  onChangeInputAmount,
}: Props) {
  return (
    <>
      <span className={styles.containerText}>
        <span>{delegate ? 'delegate to' : 'undelegate from'}</span>

        <span className={styles.moniker}>
          {moniker && moniker.length > 14
            ? `${moniker.substring(0, 14)}...`
            : moniker}
        </span>
      </span>

      <InputNumber
        value={value}
        id="myInput"
        maxValue={maxValue}
        autoFocus
        onValueChange={onChangeInputAmount}
        placeholder="amount"
        width="180px"
      />

      {/* <Denom  /> */}
    </>
  );
}

export default Delegate;
