import { convertTimestampToString } from 'src/utils/date';
import { formatNumber } from 'src/utils/utils';
import styles from './time.module.scss';

function Time({ msTime }: { msTime: number }) {
  const [valueTime, prefixTime] = convertTimestampToString(msTime).split(' ');

  return (
    <span className={styles.wrapper}>
      <span>{formatNumber(valueTime)}</span>
      <span className={styles.prefix}>{prefixTime}</span>
    </span>
  );
}

export default Time;
