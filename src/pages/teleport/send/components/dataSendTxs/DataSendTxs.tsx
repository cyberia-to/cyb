import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { CreatedAt } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { Colors } from 'src/components/containerGradient/types';
import { Nullable } from 'src/types';
import { AccountValue } from 'src/types/defaultAccount';
import { v4 as uuidv4 } from 'uuid';
import InfiniteScrollDataTsx from '../../../components/InfiniteScrollDataTxs/InfiniteScrollDataTsx';
import styles from './DataSendTxs.module.scss';
import { AmountDenomColor, Memo } from './DataSendTxsItems';
import { DataSendTxs } from './type';

function DataSendTxs({
  dataSendTxs,
  accountUser,
}: {
  dataSendTxs: DataSendTxs;
  accountUser: Nullable<AccountValue>;
}) {
  const { data, fetchNextPage, hasNextPage } = dataSendTxs;

  const itemRows = useMemo(() => {
    if (data) {
      return data.map((item) => {
        const key = uuidv4();

        const { memo } = item.tx.body;
        let isReceive = false;

        const typeTx = item.tx.body.messages[0]['@type'];
        if (
          typeTx.includes('MsgSend') &&
          item.tx.body.messages[0].to_address === accountUser?.bech32
        ) {
          isReceive = true;
        }

        return (
          <Link to={`/network/bostrom/tx/${item.txhash}`} key={`${item.txhash}_${key}`}>
            <Display
              sideSaber={isReceive ? 'left' : 'right'}
              color={item.code === 0 ? Colors.BLUE : Colors.RED}
            >
              <div className={styles.containerDataItem}>
                <Memo memo={memo} receive={isReceive} />
                <div className={styles.containerAmountTime}>
                  <AmountDenomColor receive={isReceive} coins={item.tx.body.messages[0].amount} />
                  <CreatedAt timeAt={item.timestamp} />
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
      fetchNextPage();
    }, 250);
  };

  return (
    <InfiniteScrollDataTsx
      dataLength={Object.keys(itemRows).length}
      next={fetchNextPageFnc}
      hasMore={hasNextPage}
    >
      {itemRows.length > 0 && itemRows}
    </InfiniteScrollDataTsx>
  );
}

export default DataSendTxs;
