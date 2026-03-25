import dateFormat from 'dateformat';
import { Tooltip } from 'src/components';
import styles from './DateTitle.module.scss';

function DateTitle({ date }: { date: Date }) {
  return (
    <p className={styles.date}>
      <Tooltip tooltip={dateFormat(date, 'dd/mm/yyyy')}>{dateFormat(date, 'mmm dd')}</Tooltip>
    </p>
  );
}

export default DateTitle;
