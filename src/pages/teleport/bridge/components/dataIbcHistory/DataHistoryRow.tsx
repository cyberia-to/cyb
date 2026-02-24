import { CreatedAt } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { HistoriesItem } from '../../../../../features/ibc-history/HistoriesItem';
import useGetStatus from '../../../../../features/ibc-history/useGetStatus';
import { AmountSend, RouteAddress, Status, TypeTsx } from './DataHistoryItems';
import styles from './DataHistoryRow.module.scss';

function DataHistoryRow({ item }: { item: HistoriesItem }) {
  const statusTrace = useGetStatus(item);

  return (
    <Display>
      <div className={styles.containerDataIbc}>
        <div className={styles.containerDataIbcFlexBetween}>
          <TypeTsx sourceChainId={item.sourceChainId} />

          <RouteAddress item={item} />
        </div>

        <div className={styles.containerDataIbcFlexBetween}>
          <AmountSend coin={item.amount} sourceChainId={item.sourceChainId} />

          <div className={styles.containerDataIbcTimeStatus}>
            <Status status={statusTrace} />

            <CreatedAt timeAt={item.createdAt} />
          </div>
        </div>
      </div>
    </Display>
  );
}

export default DataHistoryRow;
