import cx from 'classnames';
import { StatusOrder } from 'src/pages/Energy/redux/utils';
import styles from './Progress.module.scss';

function PillItem({
  done,
  color,
}: {
  color?: 'blue' | 'yellow';
  done?: boolean;
}) {
  return (
    <div
      className={cx(styles.pillItem, color ? styles[color] : undefined, {
        [styles.done]: done,
      })}
    />
  );
}

function Line({ status, done }: { status?: 'half' | 'full'; done?: boolean }) {
  return (
    <div className={styles.lineContainer}>
      <div
        className={cx(
          styles.lineContainerProgress,
          status ? styles[status] : undefined,
          { [styles.greenLine]: done }
        )}
      />
    </div>
  );
}

const mapStatus = (status: StatusOrder) => {
  if (status === StatusOrder.SELECT_PACK) {
    return undefined;
  }

  if (status === StatusOrder.SWAP) {
    return 'half';
  }

  return 'full';
};

function Progress({ status }: { status: StatusOrder }) {
  return (
    <div className={styles.progressContainer}>
      <PillItem color="blue" done={status !== StatusOrder.SELECT_PACK} />
      <PillItem
        color={status === StatusOrder.SWAP ? 'yellow' : undefined}
        done={status !== StatusOrder.SELECT_PACK && status !== StatusOrder.SWAP}
      />
      <PillItem
        color={
          status === StatusOrder.SEND_IBC || status === StatusOrder.STATUS_IBC
            ? 'yellow'
            : undefined
        }
        done={status === StatusOrder.FINISH_IBC}
      />
      <Line
        status={mapStatus(status)}
        done={status === StatusOrder.FINISH_IBC}
      />
    </div>
  );
}

export default Progress;
