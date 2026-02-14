import { useMessagesByAddressQuery } from 'src/generated/graphql';
import { useState } from 'react';
import TableTxsInfinite from 'src/components/TableTxsInfinite/TableTxsInfinite';
import { fromBech32 } from '@cosmjs/encoding';

const LIMIT = 10;

const typeTx = `{"cosmos.gov.v1beta1.MsgDeposit", "cosmos.gov.v1beta1.MsgVote", "cosmos.gov.v1beta1.MsgSubmitProposal"}`;

function Leadership({ accountUser }: { accountUser: string }) {
  const [hasMore, setHasMore] = useState(true);
  const { loading, error, data, fetchMore } = useMessagesByAddressQuery({
    variables: {
      address: `{${fromBech32(accountUser)}}`,
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

export default Leadership;
