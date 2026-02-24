import { Log } from '@cosmjs/stargate/build/logs';
import { useEffect, useState } from 'react';
import { MessagesByAddressQueryHookResult, useMessagesByAddressQuery } from 'src/generated/graphql';
import { Nullable, Option } from 'src/types';
import { AccountValue } from 'src/types/defaultAccount';

type Txs = {
  success: boolean;
  height: number;
  logs: Log[];
  memo: string;
  block: {
    timestamp: string;
  };
};

export type ResponseTxsByType = {
  transaction_hash: string;
  type: string;
  transaction: Txs;
  value: any;
};

export interface UseGetSendTxsByAddressByType extends MessagesByAddressQueryHookResult {
  hasMore: boolean;
  fetchMoreData: () => void;
}

const limit = '5';

function useGetSendTxsByAddressByType(
  address: Nullable<AccountValue>,
  types: string
): UseGetSendTxsByAddressByType {
  const [hasMore, setHasMore] = useState(true);
  const [addressBech32, setAddressBech32] = useState<Option<string>>();
  const result = useMessagesByAddressQuery({
    variables: {
      address: `{${addressBech32}}`,
      limit,
      offset: 0,
      types: `{${types}}`,
    },
  });

  const fetchMoreData = () => {
    result.fetchMore({
      variables: {
        offset: result.data?.messages_by_address.length,
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

  useEffect(() => {
    if (address) {
      setAddressBech32(address.bech32);
    }
  }, [address]);

  return {
    ...result,
    fetchMoreData,
    hasMore,
  };
}

export default useGetSendTxsByAddressByType;
