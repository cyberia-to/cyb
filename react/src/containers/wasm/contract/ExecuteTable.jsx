import { useCallback, useMemo, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import Loader2 from 'src/components/ui/Loader2';
import { formatNumber, parseMsgContract, trimString } from '../../../utils/utils';
import { LinkCreator, LinkTx } from '../ui/ui';

function ExecuteTable({ executions }) {
  const [itemsToShow, setItemsToShow] = useState(40);

  const setNextDisplayedPalettes = useCallback(() => {
    setTimeout(() => {
      setItemsToShow(itemsToShow + 5);
    }, 250);
  }, [itemsToShow]);

  const displayedPalettes = useMemo(
    () => executions.slice(0, itemsToShow),
    [itemsToShow, executions]
  );

  const itemTable = displayedPalettes.map((item) => {
    return (
      <tr style={{ textAlign: 'center' }} key={item.key}>
        <td>{formatNumber(item.height)}</td>
        <td>
          <LinkTx txs={item.transactionId}>{trimString(item.transactionId, 3, 3)}</LinkTx>
        </td>
        <td>{Object.keys(parseMsgContract(item.msg.msg))[0]}</td>
        <td>
          <LinkCreator address={item.msg.sender}>{trimString(item.msg.sender, 10, 4)}</LinkCreator>
        </td>
      </tr>
    );
  });

  return (
    <InfiniteScroll
      dataLength={Object.keys(displayedPalettes).length}
      next={setNextDisplayedPalettes}
      hasMore={itemsToShow < executions.length}
      loader={<Loader2 />}
    >
      <table style={{ width: '100%' }}>
        <thead>
          <tr>
            <th scope="col">Height</th>
            <th scope="col">Txs</th>
            <th scope="col">Action</th>
            <th scope="col">Sender</th>
          </tr>
        </thead>

        {itemTable.length > 0 && <tbody>{itemTable}</tbody>}
      </table>

      {!itemTable.length && (
        <p
          style={{
            textAlign: 'center',
            margin: '15px 0',
          }}
        >
          No executions
        </p>
      )}
    </InfiniteScroll>
  );
}

export default ExecuteTable;
