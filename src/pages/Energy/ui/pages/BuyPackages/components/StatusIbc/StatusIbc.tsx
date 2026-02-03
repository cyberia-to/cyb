import { useEffect, useState } from 'react';
import {
  HistoriesItem,
  StatusTx,
} from 'src/features/ibc-history/HistoriesItem';
import { useIbcHistory } from 'src/features/ibc-history/historyContext';
import useGetStatus from 'src/features/ibc-history/useGetStatus';
import {
  setIbcResult,
  setStatusOrder,
} from 'src/pages/Energy/redux/energy.redux';
import { StatusOrder } from 'src/pages/Energy/redux/utils';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import cx from 'classnames';
import styles from './StatusIbc.module.scss';

function StatusIbc() {
  const dispatch = useAppDispatch();
  const { ibcResult } = useAppSelector((state) => state.energy);
  const { getItemByTxHash } = useIbcHistory();

  const [itemByTxHash, setItemByTxHash] =
    useState<Option<HistoriesItem>>(undefined);

  const statusTrace = useGetStatus(itemByTxHash);

  useEffect(() => {
    if (!ibcResult) {
      return;
    }

    if (ibcResult.status === StatusTx.PENDING) {
      getItemByTxHash(ibcResult.ibcHash).then((item) => {
        setItemByTxHash(item[0]);
      });
    }
  }, [getItemByTxHash, JSON.stringify(ibcResult)]);

  useEffect(() => {
    if (!ibcResult) {
      return;
    }

    if (ibcResult.status === StatusTx.COMPLETE) {
      return;
    }

    if (statusTrace !== StatusTx.PENDING) {
      console.log('statusTrace', statusTrace);
      dispatch(
        setIbcResult({ ibcHash: ibcResult.ibcHash, status: statusTrace })
      );

      if (statusTrace === StatusTx.COMPLETE) {
        dispatch(setStatusOrder(StatusOrder.FINISH_IBC));
      }
    }
  }, [dispatch, JSON.stringify(ibcResult), statusTrace]);

  const status =
    ibcResult && ibcResult.status === StatusTx.COMPLETE
      ? StatusTx.COMPLETE
      : statusTrace;

  return (
    <span className={cx(styles.statusColor, styles[status])}>{status}</span>
  );
}

export default StatusIbc;
