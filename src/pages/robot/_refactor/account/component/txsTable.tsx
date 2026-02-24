import { useEffect, useState } from 'react';
import TableTxsInfinite from 'src/components/TableTxsInfinite/TableTxsInfinite';
import { useAdviser } from 'src/features/adviser/context';
import { useMessagesByAddressQuery } from 'src/generated/graphql';
import { useRobotContext } from 'src/pages/robot/robot.context';

const limit = 10; // Use a constant for the limit

function TxsTable() {
  const { address: accountUser } = useRobotContext();
  const [hasMore, setHasMore] = useState(true);
  const { setAdviser } = useAdviser();
  const { data, loading, fetchMore, error } = useMessagesByAddressQuery({
    variables: {
      address: `{${accountUser}}`,
      limit,
      offset: 0,
      types: '{}',
    },
    notifyOnNetworkStatusChange: true,
  });

  useEffect(() => {
    setAdviser(
      <>
        the life history of one neuron, which cannot be removed or destroyed <br />
        what is written in the blockchain cannot be cut out with an axe
      </>
    );
  }, [setAdviser]);

  const fetchMoreData = () => {
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
      response={{ data, error, loading }}
      hasMore={hasMore}
      fetchMoreData={fetchMoreData}
      accountUser={accountUser}
    />
  );
}

export default TxsTable;
