import { useEffect, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import { matchPath, useLocation, useParams, useSearchParams } from 'react-router-dom';
import { ActionBar, Button } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import Spark from 'src/components/search/Spark/Spark';
import Loader2 from 'src/components/ui/Loader2';
import { useDevice } from 'src/contexts/device';

import useIsOnline from 'src/hooks/useIsOnline';
import { routes } from 'src/routes';
import { IpfsContentType } from 'src/services/ipfs/types';
import { getSearchQuery } from 'src/utils/search/utils';
import FirstItems from './_FirstItems.refactor';
import ActionBarContainer from './ActionBarContainer';
import { initialContentTypeFilterState } from './constants';
import Filters from './Filters/Filters';
import useSearchData from './hooks/useSearchData';
import LLMSpark, { useIsLLMPageParam } from './LLMSpark/LLMSpark';
import styles from './SearchResults.module.scss';
import { LinksTypeFilter, SortBy } from './types';

const sortByLSKey = 'search-sort';
const NEURON_SEARCH_KEY = 'neuron';

type Props = {
  query?: string;
  noCommentText?: React.ReactNode;
  actionBarTextBtn?: string;
};

function SearchResults({ query: propQuery, noCommentText, actionBarTextBtn }: Props) {
  const { query: q, cid } = useParams();

  const [searchParams, setSearchParams] = useSearchParams();
  const [neuron, setNeuron] = useState(searchParams.get(NEURON_SEARCH_KEY));

  const isLLM = useIsLLMPageParam();

  const location = useLocation();

  const query = propQuery || q || cid || '';
  const isOnline = useIsOnline();

  const [keywordHash, setKeywordHash] = useState('');
  console.debug(query, keywordHash);

  const [rankLink, setRankLink] = useState(null);

  const [contentType, setContentType] = useState<{
    [key: string]: IpfsContentType;
  }>({});

  const [contentTypeFilter, setContentTypeFilter] = useState(initialContentTypeFilterState);
  const [sortBy, setSortBy] = useState(
    neuron ? SortBy.date : (localStorage.getItem(sortByLSKey) as SortBy | null) || SortBy.rank
  );

  const [linksTypeFilter, setLinksTypeFilter] = useState(LinksTypeFilter.all);

  const noResultsText = isOnline
    ? noCommentText || (
        <>
          there are no answers or questions to this particle {neuron && 'for this neuron'}
          <br /> be the first and create one
        </>
      )
    : "ther's nothing to show, wait until you're online";

  const {
    data: items,
    total,
    error,
    hasMore,
    isInitialLoading,
    refetch,
    fetchNextPage: next,
  } = useSearchData(keywordHash, neuron, {
    sortBy,
    linksType: linksTypeFilter,
  });

  const { isMobile: mobile } = useDevice();

  // useEffect(() => {
  //   if (query.match(/\//g)) {
  //     navigate(`/search/${replaceSlash(query)}`);
  //   }
  //   // eslint-disable-next-line react-hooks/exhaustive-deps
  // }, [query]);

  useEffect(() => {
    setContentTypeFilter(initialContentTypeFilterState);
    setContentType({});

    (async () => {
      const keywordHash = await getSearchQuery(query);

      setKeywordHash(keywordHash);
    })();
  }, [query]);

  const onClickRank = async (key) => {
    if (rankLink === key) {
      setRankLink(null);
    } else {
      setRankLink(key);
    }
  };

  const renderItems = items
    .filter((item) => {
      const { cid } = item;

      if (!Object.values(contentTypeFilter).some((value) => value)) {
        return true;
      }
      if (!contentType[cid]) {
        return false;
      }
      return contentTypeFilter[contentType[cid]];
    })
    .map((key, i) => {
      return (
        <Spark
          itemData={key}
          cid={key.cid}
          key={key.cid + i}
          linkType={key.type}
          query={query}
          rankSelected={rankLink === key.cid}
          handleRankClick={onClickRank}
          handleContentType={(type) =>
            setContentType((items) => {
              return {
                ...items,
                [key.cid]: type,
              };
            })
          }
        />
      );
    });

  return (
    <>
      <Filters
        filters={contentTypeFilter}
        setFilters={setContentTypeFilter}
        filter2={sortBy}
        setFilter2={setSortBy}
        linksFilter={linksTypeFilter}
        setLinksFilter={setLinksTypeFilter}
        total={total}
        total2={items.length}
        contentType={contentType}
        neuronFilter={{
          value: neuron,
          setValue: (address) => {
            setNeuron(address);
            setSortBy(SortBy.date);

            // TODO: need to check on senate page
            if (matchPath(routes.oracle.ask.path, location.pathname)) {
              setSearchParams((prevParams) => {
                if (address) {
                  prevParams.set(NEURON_SEARCH_KEY, address);
                } else {
                  prevParams.delete(NEURON_SEARCH_KEY);
                }

                return prevParams;
              });
            }
          },
        }}
      />

      <div className={styles.search}>
        {!isLLM && <LLMSpark searchText={query} />}
        <FirstItems query={query} />

        {isInitialLoading ? (
          <Loader2 />
        ) : Object.keys(renderItems).length > 0 ? (
          <InfiniteScroll
            dataLength={items.length}
            next={next}
            className={styles.infiniteScroll}
            hasMore={hasMore}
            loader={<Loader2 />}
          >
            {renderItems}
          </InfiniteScroll>
        ) : error ? (
          <Display color="red">
            <p>{error.message}</p>
          </Display>
        ) : (
          <Display color="white">{noResultsText}</Display>
        )}
      </div>

      {!mobile && (
        <div className={styles.actionBar}>
          {isLLM ? (
            <ActionBar>
              <Button link={`${routes.studio.path}?cid=${keywordHash}`}>Edit & Cyberlink</Button>
            </ActionBar>
          ) : (
            <ActionBarContainer
              textBtn={actionBarTextBtn}
              keywordHash={keywordHash}
              update={() => {
                refetch();
                setRankLink(null);
              }}
              rankLink={rankLink}
            />
          )}
        </div>
      )}
    </>
  );
}

export default SearchResults;
