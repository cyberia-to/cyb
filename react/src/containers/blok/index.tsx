import { useMemo, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import { Link } from 'react-router-dom';
import { Dots, MainContainer, TextTable } from 'src/components';
import Table from 'src/components/Table/Table';
import { useBlocksQuery } from 'src/generated/graphql';
import { routes } from 'src/routes';
import { formatNumber, trimString } from '../../utils/utils';

const dateFormat = require('dateformat');

const limit = 20;

const TableKeyMap = {
  hash: 'hash',
  height: 'height',
  tx: 'tx',
  proposerAddress: 'proposer address',
  timestampUTC: 'timestamp, UTC',
};

function Block() {
  const [hasMore, setHasMore] = useState(true);
  const { data, loading, error, fetchMore } = useBlocksQuery({
    variables: {
      limit,
      offset: 0,
    },
  });

  const lastBlockQuery = data?.block[data.block.length - 1].height;

  const fetchMoreData = async () => {
    setTimeout(() => {
      fetchMore({
        variables: {
          offset: data?.block.length,
          where: { height: { _lte: lastBlockQuery } },
        },
        updateQuery: (prev, { fetchMoreResult }) => {
          if (!fetchMoreResult) {
            return prev;
          }
          setHasMore(fetchMoreResult.block.length > 0);

          return {
            ...prev,
            block: [...prev.block, ...fetchMoreResult.block],
          };
        },
      });
    }, 2000);
  };

  const tableData = useMemo(() => {
    return (data?.block || []).map((item) => {
      return {
        hash: <TextTable>{trimString(item.hash, 5, 5)}</TextTable>,
        height: (
          <TextTable>
            <Link to={routes.blocks.idBlock.getLink(item.height)}>{formatNumber(item.height)}</Link>
          </TextTable>
        ),
        tx: (
          <TextTable>{formatNumber(item.transactions_aggregate.aggregate?.count || 0)}</TextTable>
        ),
        proposerAddress: <TextTable>{trimString(item.proposer_address, 5, 5)}</TextTable>,
        timestampUTC: <TextTable>{dateFormat(item.timestamp, 'dd/mm/yyyy, HH:MM:ss')}</TextTable>,
      };
    });
  }, [data]);

  if (loading) {
    return (
      <div
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <Dots />
      </div>
    );
  }

  if (error) {
    return <span>{error.message}</span>;
  }

  return (
    <MainContainer>
      <InfiniteScroll
        dataLength={data?.block.length || 0}
        next={fetchMoreData}
        refreshFunction={fetchMoreData}
        hasMore={hasMore}
        loader={
          <h4>
            Loading
            <Dots />
          </h4>
        }
      >
        <Table
          data={tableData}
          isLoading={loading}
          columns={Object.keys(TableKeyMap).map((key) => ({
            header: TableKeyMap[key],
            accessorKey: key,
            cell: (info) => info.getValue(),
          }))}
        />
      </InfiniteScroll>
    </MainContainer>
  );
}

export default Block;
