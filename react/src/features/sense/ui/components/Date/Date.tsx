import cx from 'classnames';
import dateFormat from 'dateformat';
import { Tooltip } from 'src/components';
import styles from './Date.module.scss';

type Props = {
  timestamp: string;
  className?: string;

  // temp prop, refactor
  timeOnly?: boolean;
};

function Date2({ timestamp, className, timeOnly }: Props) {
  const date = new Date(timestamp);
  const today = new Date();

  const isToday =
    date.getDate() === today.getDate() &&
    date.getMonth() === today.getMonth() &&
    date.getFullYear() === today.getFullYear();

  // check if more than half year ago
  const isCurrentYear = date.getFullYear() === today.getFullYear();
  const isHalfYearAgo = date.getTime() < today.getTime() - 1000 * 60 * 60 * 24 * 180;

  let schema = 'dd/mm';

  if (isToday || timeOnly) {
    schema = 'HH:MM';
  } else if (!isCurrentYear && isHalfYearAgo) {
    schema = 'mm/yyyy';
  }

  return (
    <div className={cx(styles.date, className)}>
      <Tooltip tooltip={dateFormat(date, 'dd/mm/yyyy HH:MM')}>
        <time>{dateFormat(date, schema)}</time>
      </Tooltip>
    </div>
  );
}

export default Date2;
