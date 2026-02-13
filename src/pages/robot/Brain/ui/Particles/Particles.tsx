import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import {
  ParticlesQuery,
  useCyberlinksCountByNeuron2Query,
  useParticlesAggregateQuery,
  useParticlesQuery,
} from 'src/generated/graphql';
import { useRobotContext } from 'src/pages/robot/robot.context';

import { createColumnHelper } from '@tanstack/react-table';
import Table from 'src/components/Table/Table';
import ContentItem from 'src/components/ContentItem/contentItem';
import { Display } from 'src/components';
import { useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import Loader2 from 'src/components/ui/Loader2';
import ParticleSize from 'src/features/particle/ParticleSize/ParticleSize';
import { Link, useNavigate } from 'react-router-dom';
import { routes } from 'src/routes';
import useRank from 'src/features/cyberlinks/rank/useRank';
import styles from './Particles.module.scss';

const columnHelper = createColumnHelper<ParticlesQuery['particles'][0]>();

const LIMIT = 20;

function Rank2({ cid }) {
  const rank = useRank(cid);

  if (!rank) {
    return null;
  }

  return (
    <span
      style={{
        fontSize: 14,
      }}
    >
      {rank / 10 ** 15}
      {/* {rank.toLocaleString().replaceAll(',', ' ')} */}
    </span>
  );
}

function Particles() {
  const { address } = useRobotContext();
  const [hasMore, setHasMore] = useState(true);
  // const [offset, setOffset] = useState(0);

  const navigate = useNavigate();

  const { data, loading, error, fetchMore } = useParticlesQuery({
    variables: {
      neuron: address,
      limit: LIMIT,
    },
  });

  console.log(data);

  const particleAggregateQuery = useParticlesAggregateQuery({
    variables: {
      neuron: address,
    },
  });

  const cyberlinksCountQuery = useCyberlinksCountByNeuron2Query({
    variables: {
      address,
    },
  });

  function fetch() {
    fetchMore({
      variables: {
        offset: data?.particles.length,
      },
      updateQuery: (prev, { fetchMoreResult }) => {
        if (!fetchMoreResult) {
          return prev;
        }

        setHasMore(fetchMoreResult.particles.length > 0);

        return {
          ...prev,
          particles: [...prev.particles, ...fetchMoreResult.particles],
        };
      },
    });
  }

  useAdviserTexts({
    defaultText: 'Particles',
    isLoading: loading,
    error,
  });

  console.log(data, cyberlinksCountQuery);

  const total =
    particleAggregateQuery.data?.particles_aggregate.aggregate?.count;

  return (
    <div
      style={{
        position: 'relative',
        zIndex: 2,
      }}
    >
      <Display>
        <p>{total} particles</p>

        <p>
          {cyberlinksCountQuery.data?.cyberlinks_aggregate.aggregate?.count}{' '}
          cyberlinks
        </p>
      </Display>
      <br />
      <Display noPadding>
        <InfiniteScroll
          dataLength={data?.particles.length || 0}
          next={fetch}
          hasMore={hasMore}
          loader={<Loader2 />}
        >
          <Table
            onSelect={(row) => {
              const cid = data!.particles[row].particle;

              navigate(routes.oracle.ask.getLink(cid));
            }}
            columns={[
              columnHelper.accessor('particle', {
                header: 'CID',
                id: 'cid',
                cell: (info) => {
                  return (
                    <ContentItem
                      cid={info.getValue()}
                      className={styles.particleContent}
                    />
                  );
                },
              }),
              columnHelper.accessor('timestamp', {
                header: 'timestamp',
                cell: (info) => {
                  const hash = info.row.original.transaction_hash;
                  // return new Date(info.getValue()).toLocaleString();
                  return (
                    <Link to={routes.txExplorer.getLink(hash)}>
                      {new Intl.DateTimeFormat(navigator.language, {
                        dateStyle: 'medium',
                        timeStyle: 'short',
                      }).format(new Date(info.getValue()))}
                    </Link>
                  );
                },
              }),
              columnHelper.accessor('particle', {
                header: 'size',
                id: 'size',
                cell: (info) => {
                  return <ParticleSize cid={info.getValue()} />;
                },
              }),
              columnHelper.accessor('particle', {
                header: (
                  <>
                    {/* rank{' '} */}
                    {/* <Link
                      to="https://docs.cyb.ai/#/page/cyberank"
                      replace
                      target="_blank"
                    >
                      🦠
                    </Link> */}
                    probability of observation
                  </>
                ),
                id: 'rank',
                cell: (info) => {
                  return <Rank2 cid={info.getValue()} />;
                },
              }),
            ]}
            data={data?.particles || []}
          />
        </InfiniteScroll>
      </Display>
    </div>
  );
}

export default Particles;
