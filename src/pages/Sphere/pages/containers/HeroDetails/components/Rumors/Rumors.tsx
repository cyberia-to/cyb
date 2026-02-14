import { useMessagesByAddressQuery } from 'src/generated/graphql';
import { useState } from 'react';

import TableTxsInfinite from 'src/components/TableTxsInfinite/TableTxsInfinite';

const LIMIT = 10;

const typeTx = `{"cosmos.staking.v1beta1.MsgDelegate", "cosmos.staking.v1beta1.MsgUndelegate", "cosmos.staking.v1beta1.MsgBeginRedelegate"}`;

function Rumors({ accountUser }: { accountUser: string }) {
  const [hasMore, setHasMore] = useState(true);
  const { loading, error, data, fetchMore } = useMessagesByAddressQuery({
    variables: {
      address: `{${accountUser}}`,
      types: typeTx,
      limit: LIMIT,
      offset: 0,
    },
  });

  const fetchNextData = () => {
    fetchMore({
      variables: {
        offset: data?.messages_by_address.length,
      },
      updateQuery: (prev, { fetchMoreResult }) => {
        if (!fetchMoreResult) {
          return prev;
        }
        setHasMore(fetchMoreResult.messages_by_address.length > 0);

        return {
          ...prev,
          messages_by_address: [
            ...prev.messages_by_address,
            ...fetchMoreResult.messages_by_address,
          ],
        };
      },
    });
  };

  return (
    <TableTxsInfinite
      response={{ data, loading, error }}
      accountUser={accountUser}
      hasMore={hasMore}
      fetchMoreData={fetchNextData}
    />
  );
}

export default Rumors;
