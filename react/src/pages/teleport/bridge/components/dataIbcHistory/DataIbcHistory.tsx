import { useCallback, useMemo, useState } from 'react';
import { useIbcHistory } from '../../../../../features/ibc-history/historyContext';
import InfiniteScrollDataTsx from '../../../components/InfiniteScrollDataTxs/InfiniteScrollDataTsx';
import DataHistoryRow from './DataHistoryRow';

function DataIbcHistory() {
  const { ibcHistory } = useIbcHistory();
  const [itemsToShow, setItemsToShow] = useState(5);

  const setNextDisplayed = useCallback(() => {
    setTimeout(() => {
      setItemsToShow(itemsToShow + 5);
    }, 250);
  }, [itemsToShow]);

  const displayedPalettes = useMemo(() => {
    if (ibcHistory) {
      return ibcHistory.slice(0, itemsToShow);
    }
    return [];
  }, [itemsToShow, ibcHistory]);

  const ItemRows = useMemo(() => {
    return displayedPalettes.map((item) => {
      return <DataHistoryRow key={item.txHash} item={item} />;
    });
  }, [displayedPalettes]);

  const hasMore = useMemo(() => {
    return Boolean(ibcHistory && itemsToShow < ibcHistory.length);
  }, [ibcHistory, itemsToShow]);

  return (
    <InfiniteScrollDataTsx dataLength={ItemRows.length} next={setNextDisplayed} hasMore={hasMore}>
      {ItemRows.length > 0 && ItemRows}
    </InfiniteScrollDataTsx>
  );
}

export default DataIbcHistory;
