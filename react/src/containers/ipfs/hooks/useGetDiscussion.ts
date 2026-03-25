import { useEffect, useState } from 'react';
import { LinksType, LinksTypeFilter } from 'src/containers/Search/types';
import useCyberlinksCount from 'src/features/cyberlinks/hooks/useCyberlinksCount';
import { Order_By as OrderBy, useCyberlinksByParticleQuery } from 'src/generated/graphql';

const limit = 15;

type Props = {
  hash: string;
  type: LinksType;
  neuron: string | null;
};

function useGetLinks({ hash, type = LinksTypeFilter.from, neuron }: Props, { skip = false } = {}) {
  const where = {
    [`particle_${type}`]: { _eq: hash },
  };

  if (neuron) {
    where.neuron = { _eq: neuron };
  }

  // always no next page when skip
  const [hasNextPage, setHasNextPage] = useState(!skip);
  const [isInitialLoading, setIsInitialLoading] = useState(true);
  const {
    loading: isFetching,
    error,
    data,
    networkStatus: status,
    refetch,
    fetchMore,
  } = useCyberlinksByParticleQuery({
    variables: {
      where,
      orderBy: { timestamp: OrderBy.Desc },
      limit,
    },
    skip: skip || !hash,
  });

  useEffect(() => {
    if (!skip) {
      setHasNextPage(true);
    }
  }, [skip]);

  useEffect(() => {
    isInitialLoading && setIsInitialLoading(false);
  }, [isInitialLoading]);

  const fetchNextPage = () => {
    fetchMore({
      variables: {
        offset: data?.cyberlinks.length,
      },
      updateQuery: (prev, { fetchMoreResult }) => {
        if (!fetchMoreResult) {
          setHasNextPage(false);
          return prev;
        }

        setHasNextPage(fetchMoreResult?.cyberlinks.length >= limit);

        return {
          ...prev,
          cyberlinks: [...prev.cyberlinks, ...fetchMoreResult.cyberlinks],
        };
      },
    });
  };

  const cyberlinksCountQuery = useCyberlinksCount(hash, neuron);
  const total = cyberlinksCountQuery.data[type];
  const particles = (data?.cyberlinks || []).map((item) => {
    return {
      cid: item[type === 'from' ? 'to' : 'from'],
      type,
      timestamp: item.timestamp,
    };
  });

  return {
    status,
    data: particles,
    error,
    isFetching,
    fetchNextPage,
    isInitialLoading,
    hasNextPage,
    total,
    refetch,
  };
}

export default useGetLinks;
