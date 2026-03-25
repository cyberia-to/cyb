import { useMemo } from 'react';
import { UseGetSendTxsByAddressByType } from 'src/pages/teleport/hooks/useGetSendTxsByAddress';
import { v4 as uuidv4 } from 'uuid';
import InfiniteScrollDataTsx from '../../../components/InfiniteScrollDataTxs/InfiniteScrollDataTsx';
import DataSwapTxsItem from './DataSwapTxsItem';

function DataSwapTxs({ dataTxs }: { dataTxs: UseGetSendTxsByAddressByType }) {
  const { data, error, hasMore, fetchMoreData } = dataTxs;

  const itemRows = useMemo(() => {
    if (data) {
      return data.messages_by_address.map((item) => {
        const key = uuidv4();

        return <DataSwapTxsItem item={item} key={`${item.transaction_hash}_${key}`} />;
      });
    }

    return [];
  }, [data]);

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

export default DataSwapTxs;
