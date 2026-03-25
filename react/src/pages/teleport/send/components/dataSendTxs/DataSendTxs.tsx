import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { CreatedAt } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { Colors } from 'src/components/containerGradient/types';
import { UseGetSendTxsByAddressByType } from 'src/pages/teleport/hooks/useGetSendTxsByAddress';
import { Nullable } from 'src/types';
import { AccountValue } from 'src/types/defaultAccount';
import { v4 as uuidv4 } from 'uuid';
import InfiniteScrollDataTsx from '../../../components/InfiniteScrollDataTxs/InfiniteScrollDataTsx';
import styles from './DataSendTxs.module.scss';
import { AmountDenomColor, Memo } from './DataSendTxsItems';

function DataSendTxs({
  dataSendTxs,
  accountUser,
}: {
  dataSendTxs: UseGetSendTxsByAddressByType;
  accountUser: Nullable<AccountValue>;
}) {
  const { data, error, hasMore, fetchMoreData } = dataSendTxs;

  const itemRows = useMemo(() => {
    if (data) {
      return data.messages_by_address.map((item) => {
        const key = uuidv4();

        const memo = item.transaction?.memo || '';
        const isReceive = item.value?.to_address === accountUser?.bech32;
        const coins = item.value?.amount || [];

        return (
          <Link to={`/network/bostrom/tx/${item.transaction_hash}`} key={`${item.transaction_hash}_${key}`}>
            <Display
              sideSaber={isReceive ? 'left' : 'right'}
              color={item.transaction?.success ? Colors.BLUE : Colors.RED}
            >
              <div className={styles.containerDataItem}>
                <Memo memo={memo} receive={isReceive} />
                <div className={styles.containerAmountTime}>
                  <AmountDenomColor receive={isReceive} coins={coins} />
                  <CreatedAt timeAt={item.transaction?.block.timestamp} />
                </div>
              </div>
            </Display>
          </Link>
        );
      });
    }

    return [];
  }, [data, accountUser]);

  const fetchNextPageFnc = () => {
    setTimeout(() => {
      fetchMoreData();
    }, 250);
  };

  return (
    <InfiniteScrollDataTsx
      dataLength={Object.keys(itemRows).length}
      next={fetchNextPageFnc}
      hasMore={hasMore}
    >
      {error ? <span>Error: {error.message}</span> : itemRows.length > 0 ? itemRows : null}
    </InfiniteScrollDataTsx>
  );
}

export default DataSendTxs;
