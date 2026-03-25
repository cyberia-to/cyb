import { TableEv as Table } from '@cybercongress/gravity';
import { useCallback, useMemo, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import { Link } from 'react-router-dom';
import { Cid, Dots, NoItems, TextTable } from '../../../../../components';
import { trimString } from '../../../../../utils/utils';

const dateFormat = require('dateformat');

function TableLink({ data }) {
  const [itemsToShow, setItemsToShow] = useState(40);

  const setNextDisplayedPalettes = useCallback(() => {
    setTimeout(() => {
      setItemsToShow(itemsToShow + 5);
    }, 250);
  }, [itemsToShow]);

  const displayedPalettes = useMemo(
    () => data.slice(0, itemsToShow),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [itemsToShow, data.slice]
  );

  const validatorRows = displayedPalettes.map((item, i) => (
    <Table.Row borderBottom="none" display="flex" key={`${item.transaction_hash}_${i}`}>
      <Table.TextCell textAlign="center">
        <TextTable>
          <Link to={`/network/bostrom/tx/${item.transaction_hash}`}>
            {trimString(item.transaction_hash, 6, 6)}
          </Link>
        </TextTable>
      </Table.TextCell>
      <Table.TextCell flex={1.5} textAlign="center">
        <TextTable>{dateFormat(item.timestamp, 'dd/mm/yyyy, HH:MM:ss')}</TextTable>
      </Table.TextCell>
      <Table.TextCell textAlign="center">
        <TextTable>
          <Cid cid={item.particle_from}>{trimString(item.particle_from, 6, 6)}</Cid>
        </TextTable>
      </Table.TextCell>
      <Table.TextCell textAlign="center">
        <TextTable>
          <Cid cid={item.particle_to}>{trimString(item.particle_to, 6, 6)}</Cid>
        </TextTable>
      </Table.TextCell>
    </Table.Row>
  ));

  return (
    <div>
      <Table>
        <Table.Head
          style={{
            backgroundColor: '#000',
            borderBottom: '1px solid #ffffff80',
            marginTop: '10px',
            paddingBottom: '10px',
          }}
        >
          <Table.TextHeaderCell textAlign="center">
            <TextTable>tx</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell flex={1.5} textAlign="center">
            <TextTable>timestamp, UTC</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell textAlign="center">
            <TextTable>from</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell textAlign="center">
            <TextTable>to</TextTable>
          </Table.TextHeaderCell>
        </Table.Head>
        <Table.Body
          style={{
            backgroundColor: '#000',
            overflowY: 'hidden',
            padding: 7,
          }}
        >
          <InfiniteScroll
            dataLength={Object.keys(displayedPalettes).length}
            next={setNextDisplayedPalettes}
            hasMore={data.length !== Object.keys(displayedPalettes).length}
            loader={
              <h4>
                Loading
                <Dots />
              </h4>
            }
            pullDownToRefresh
            pullDownToRefreshContent={
              <h3 style={{ textAlign: 'center' }}>&#8595; Pull down to refresh</h3>
            }
            releaseToRefreshContent={
              <h3 style={{ textAlign: 'center' }}>&#8593; Release to refresh</h3>
            }
            refreshFunction={setNextDisplayedPalettes}
          >
            {data.length > 0 ? validatorRows : <NoItems text="No cyberLinks" />}
          </InfiniteScroll>
        </Table.Body>
      </Table>
    </div>
  );
}

export default TableLink;
