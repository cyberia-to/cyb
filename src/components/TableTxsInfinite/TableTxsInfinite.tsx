import { ApolloError } from '@apollo/client';
import InfiniteScroll from 'react-infinite-scroll-component';
import { MessagesByAddressQuery } from 'src/generated/graphql';
import Display from '../containerGradient/Display/Display';
import TableDataTxs from './component/TableDataTxs';
import styles from './TableTxsInfinite.module.scss';

type Props = {
  hasMore: boolean;
  fetchMoreData: () => void;
  accountUser: string | null;
  response: {
    data: MessagesByAddressQuery | undefined;
    error: ApolloError | undefined;
    loading: boolean;
  };
};

function TableTxsInfinite({ hasMore, fetchMoreData, response, accountUser }: Props) {
  const { data, loading, error } = response;

  return (
    <Display noPadding>
      <InfiniteScroll
        dataLength={(data?.messages_by_address || []).length}
        next={fetchMoreData}
        hasMore={hasMore}
        loader={false}
        className={styles.infiniteScroll}
      >
        <TableDataTxs
          loading={loading}
          data={data?.messages_by_address || []}
          accountUser={accountUser || ''}
        />
      </InfiniteScroll>

      {error && <span>Error: {error.message}</span>}
    </Display>
  );
}

export default TableTxsInfinite;
