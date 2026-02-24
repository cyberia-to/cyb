import { useInfiniteQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { enqueueLinksSave } from 'src/services/backend/channels/BackendQueueChannel/backendQueueSenders';
import { mapLinkToLinkDto } from 'src/services/CozoDb/mapping';
import { searchByHash } from 'src/utils/search/utils';
import { reduceParticleArr } from './useGetBackLink';

function useGetAnswers(hash) {
  const queryClient = useQueryClient();
  const [total, setTotal] = useState(0);
  const { status, data, error, isFetching, fetchNextPage, hasNextPage, refetch } = useInfiniteQuery(
    ['useGetAnswers', hash],
    async ({ pageParam = 0 }) => {
      const response = await searchByHash(queryClient, hash, pageParam);
      const result = response?.result || [];
      const reduceArr = result ? reduceParticleArr(result) : [];
      setTotal(pageParam === 0 && response.pagination.total);

      enqueueLinksSave(result.map((l) => mapLinkToLinkDto(hash, l.particle)));
      return { data: reduceArr, page: pageParam };
    },
    {
      enabled: Boolean(queryClient),
      getNextPageParam: (lastPage) => {
        if (lastPage.data && lastPage.data.length === 0) {
          return undefined;
        }

        const nextPage = lastPage.page !== undefined ? lastPage.page + 1 : 0;
        return nextPage;
      },
    }
  );

  return {
    status,
    data,
    error,
    isFetching,
    fetchNextPage,
    hasNextPage,
    total,
    refetch,
  };
}

export default useGetAnswers;
