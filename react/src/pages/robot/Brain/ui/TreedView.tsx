import { useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import Loader2 from 'src/components/ui/Loader2';
import { Order_By as OrderBy, useCyberlinksByParticleQuery } from 'src/generated/graphql';
import { v4 as uuidv4 } from 'uuid';
import { LIMIT_TREED } from '../utils';
import TreedItem from './TreedItem/TreedItem';
import styles from './TreedView.modile.scss';

function TreedView({ address }: { address?: string }) {
  const [hasMore, setHasMore] = useState(true);

  let where;
  if (address) {
    where = { neuron: { _eq: address } };
  } else {
    where = {};
  }

  const { loading, error, data, fetchMore } = useCyberlinksByParticleQuery({
    variables: {
      where,
      orderBy: { height: OrderBy.Desc },
      limit: LIMIT_TREED,
    },
  });

  const fetchMoreData = async () => {
    setTimeout(() => {
      fetchMore({
        variables: {
          limit: LIMIT_TREED,
          offset: data?.cyberlinks.length,
        },
        updateQuery: (prev, { fetchMoreResult }) => {
          if (!fetchMoreResult) {
            return prev;
          }

          setHasMore(fetchMoreResult.cyberlinks.length > 0);

          return {
            ...prev,
            cyberlinks: [...prev.cyberlinks, ...fetchMoreResult.cyberlinks],
          };
        },
      });
    }, 2000);
  };

  return (
    <InfiniteScroll
      dataLength={data?.cyberlinks.length || 0}
      next={fetchMoreData}
      hasMore={hasMore}
      loader={<Loader2 />}
      className={styles.wrapper}
    >
      {data?.cyberlinks.map((item) => {
        const key = uuidv4();

        return (
          <TreedItem key={key} link={{ from: item.from, to: item.to }} address={address || ''} />
        );
      })}
    </InfiniteScroll>
  );
}

export default TreedView;
