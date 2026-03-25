import { useInfiniteQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { request } from 'graphql-request';
import { INDEX_HTTPS } from 'src/constants/config';
import {
  MessagesByAddressDocument,
  MessagesByAddressQuery,
  MessagesByAddressQueryVariables,
} from 'src/generated/graphql';

const limit = '1000';

function useGetTsxByAddress(address: string) {
  const { status, data, error, isFetching, fetchNextPage, hasNextPage } = useInfiniteQuery(
    ['messagesByAddressGql', address],
    async ({ pageParam = 0 }) => {
      // Use the generated variable type
      const variables: MessagesByAddressQueryVariables = {
        address: `{${address}}`, // Ensure this format matches expected input
        limit: parseInt(limit, 10),
        offset: new BigNumber(limit).multipliedBy(pageParam).toNumber(),
      };
      // Directly typing the response improves type safety
      const response = await request<MessagesByAddressQuery>(
        INDEX_HTTPS,
        MessagesByAddressDocument,
        variables
      );
      return { data: response.messages_by_address, page: pageParam };
    },
    {
      getNextPageParam: (lastPage) => {
        if (lastPage.data && lastPage.data.length === 0) {
          return undefined;
        }
        const nextPage = lastPage.page !== undefined ? lastPage.page + 1 : 0;
        return nextPage;
      },
    }
  );

  return { data, error, isFetching, fetchNextPage, hasNextPage, status };
}

export default useGetTsxByAddress;
