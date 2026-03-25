import cx from 'classnames';
import { useMemo } from 'react';
import { useAppSelector } from 'src/redux/hooks';
import { v4 as uuidv4 } from 'uuid';
import styles from './TimeHistory.module.scss';
import TimeHistoryItem from './ui/TimeHistoryItem/TimeHistoryItem';

function TimeHistory() {
  const { timeHistory } = useAppSelector((state) => state.timeHistory);

  const renderItem = useMemo(() => {
    if (!timeHistory.length) {
      return null;
    }

    return [...timeHistory].slice(timeHistory.length - 5).map(({ time, action }) => {
      const key = uuidv4();

      return <TimeHistoryItem key={key} time={time} action={action} />;
    });
  }, [timeHistory]);

  return <div className={cx(styles.wrapper, styles.rotate)}>{renderItem}</div>;
}

export default TimeHistory;
