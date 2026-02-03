import { useEffect, useState } from 'react';
import { useIbcHistory } from './historyContext';
import { HistoriesItem, StatusTx } from './HistoriesItem';

function* toGenerator<R>(p: Promise<R>) {
  return (yield p) as R;
}

function useGetStatus(item?: HistoriesItem) {
  const { traceHistoryStatus, updateStatusByTxHash } = useIbcHistory();

  const [status, setStatus] = useState<StatusTx>(
    item ? item.status : StatusTx.PENDING
  );

  // eslint-disable-next-line react-hooks/exhaustive-deps
  function* tryUpdateHistoryStatus(item: HistoriesItem) {
    const status = yield* toGenerator(traceHistoryStatus(item));

    return status;
  }

  async function helper(
    func: Generator<Promise<StatusTx>, StatusTx, StatusTx>
  ): Promise<StatusTx> {
    const { value } = func.next();
    const output = await value;
    return output;
  }

  useEffect(() => {
    if (!item) {
      return;
    }
    const getValue = async () => {
      const gen = await helper(tryUpdateHistoryStatus(item));

      updateStatusByTxHash(item.txHash, gen);

      setStatus(gen);

      if (gen === StatusTx.TIMEOUT) {
        getValue();
      }
    };
    getValue();
  }, [item, tryUpdateHistoryStatus, updateStatusByTxHash]);

  return status;
}

export default useGetStatus;
