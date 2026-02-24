import { dateToUtcNumber, getNowUtcNumber } from 'src/utils/date';
import { timeSince } from 'src/utils/utils';
import styles from './CreatedAt.module.scss';

export type Props = {
  timeAt: string | number;
};

function CreatedAt({ timeAt }: Props) {
  let timeAgoInMS = 0;

  const timeUtc = typeof timeAt === 'string' ? dateToUtcNumber(timeAt) : timeAt;

  const time = getNowUtcNumber() - timeUtc;
  if (time && time > 0) {
    timeAgoInMS = time;
  }

  const timeSinceValue = timeSince(timeAgoInMS);

  return (
    <span className={styles.createdAt}>
      {timeSinceValue === 'now' ? timeSinceValue : `${timeSinceValue} ago`}
    </span>
  );
}

export default CreatedAt;
