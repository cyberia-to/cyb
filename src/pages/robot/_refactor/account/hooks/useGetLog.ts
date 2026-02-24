import { OrderBy } from '@cybercongress/cyber-ts/cosmos/tx/v1beta1/service';
import { useInfiniteQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { CID_TWEET } from 'src/constants/app';
// import { useCyberClient } from 'src/contexts/queryCyberClient';
import { getTransactions } from 'src/services/transactions/lcd';

const LIMIT = 20;

function useGetLog(address: string | null) {
  // const { rpc } = useCyberClient();
  const { data, fetchNextPage, hasNextPage, refetch, error, isInitialLoading } = useInfiniteQuery(
    ['useGetLog', address],
    async ({ pageParam = 0 }: { pageParam?: number }) => {
      // const response = await rpc.cosmos.tx.v1beta1.getTxsEvent({
      //   // pagination: { limit: LIMIT, offset },
      //   events: [
      //     `cyberlink.particleFrom='${CID_TWEET}'`,
      //     `cyberlink.neuron='${address}'`,
      //   ],
      //   orderBy: OrderBy.ORDER_BY_DESC,
      // });

      const response = await getTransactions({
        events: [
          {
            key: 'cyberlink.particleFrom',
            value: CID_TWEET,
          },
          {
            key: 'cyberlink.neuron',
            value: address,
          },
        ],
        orderBy: OrderBy.ORDER_BY_DESC,
      });

      return { data: response, page: pageParam };
    },
    {
      enabled: Boolean(address),
      getNextPageParam: (lastPage) => {
        const { page } = lastPage;

        const total = lastPage.data.pagination?.total || 0;

        if (!total || (page + 1) * LIMIT > total) {
          return undefined;
        }

        return page + 1;
      },
    }
  );

  const memoData = useMemo(() => {
    return (
      data?.pages?.reduce((acc, page) => {
        return acc.concat(page.data.txResponses);
      }, []) || []
    );
  }, [data]);

  return {
    data: memoData,
    fetchNextPage,
    hasNextPage,
    refetch,
    error,
    isInitialLoading,
  };
}

export default useGetLog;
