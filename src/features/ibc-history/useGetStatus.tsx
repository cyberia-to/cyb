import { useCallback, useEffect, useRef, useState } from 'react';
import { HistoriesItem, StatusTx } from './HistoriesItem';
import { useIbcHistory } from './historyContext';

const PENDING_RETRY_INTERVAL = 15000; // 15s between retries for stuck PENDING
const MAX_PENDING_RETRIES = 20; // give up after ~5 min

function useGetStatus(item: HistoriesItem) {
  const { traceHistoryStatus, updateStatusByTxHash } = useIbcHistory();

  const [status, setStatus] = useState(item.status);
  const retriesRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout>>();

  const trace = useCallback(async () => {
    const newStatus = await traceHistoryStatus(item);

    updateStatusByTxHash(item.txHash, newStatus);
    setStatus(newStatus);

    if (newStatus === StatusTx.TIMEOUT) {
      // Retry immediately for timeout (waiting for refund)
      retriesRef.current = 0;
      trace();
      return;
    }

    if (newStatus === StatusTx.PENDING) {
      retriesRef.current += 1;
      if (retriesRef.current < MAX_PENDING_RETRIES) {
        timerRef.current = setTimeout(trace, PENDING_RETRY_INTERVAL);
      }
    }
    // COMPLETE or REFUNDED — done, no retry
  }, [item, traceHistoryStatus, updateStatusByTxHash]);

  useEffect(() => {
    retriesRef.current = 0;
    trace();

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [trace]);

  return status;
}

export default useGetStatus;
