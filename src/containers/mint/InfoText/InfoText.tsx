import { DenomArr } from 'src/components';
import { formatNumber } from 'src/utils/utils';
import { SelectedState } from '../types';
import { SLOTS_MAX } from '../utils';
import styles from './InfoText.module.scss';

type Props = {
  value: {
    amount: number;
    days: number;
  };
  selected: SelectedState;
  resourceToken: number;
};

function InfoText({ value: { amount, days }, selected, resourceToken }: Props) {
  return (
    <p className={styles.text}>
      You’re freezing <strong>{formatNumber(amount)}</strong>{' '}
      <DenomArr denomValue="hydrogen" onlyImg /> for <strong>{days} days</strong>. It will release{' '}
      <strong>{resourceToken}</strong> <DenomArr denomValue={selected} onlyImg /> for you. At the
      end of the period, {selected} becomes liquid automatically, but you can use it to boost
      ranking during the freeze. You can have only <strong>{SLOTS_MAX} slots</strong> for investmint
      at a time.
    </p>
  );
}

export default InfoText;
