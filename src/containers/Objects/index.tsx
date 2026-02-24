import { useEffect, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import Spark from 'src/components/search/Spark/Spark';
import { Dots, Loading } from '../../components';
import { getRankGrade, getRelevance } from '../../utils/search/utils';
import { coinDecimals } from '../../utils/utils';
import { MainContainer } from '../portal/components';

function Relevance({ items, fetchMoreData }) {
  return (
    <div
      style={{
        margin: '0 auto',
      }}
    >
      <InfiniteScroll
        dataLength={Object.keys(items).length}
        next={fetchMoreData}
        hasMore
        loader={
          <h4
            style={{
              textAlign: 'center',
            }}
          >
            Loading
            <Dots />
          </h4>
        }
        refreshFunction={fetchMoreData}
      >
        {Object.keys(items).map((key) => {
          return <Spark cid={key} key={key} itemData={items[key]} query="particles" />;
        })}
      </InfiniteScroll>
    </div>
  );
}

// need refactor
function Objects() {
  const [items, setItems] = useState([]);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [allPage, setAllPage] = useState(0);

  function processData(data) {
    const links = (data.result || []).reduce(
      (obj, link) => ({
        ...obj,
        [link.particle]: {
          rank: coinDecimals(link.rank),
          particle: link.particle,
          grade: getRankGrade(coinDecimals(link.rank)),
          status: 'impossibleLoad',
          text: link.particle,
          content: false,
        },
      }),
      {}
    );

    return links;
  }

  const getFirstItem = async () => {
    const data = await getRelevance(page);
    const links = processData(data);

    setItems(links);
    setPage(page + 1);
    setLoading(false);
    setAllPage(Math.ceil(parseFloat(data.pagination?.total || '0') / 50));
  };

  useEffect(() => {
    getFirstItem();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const fetchMoreData = async () => {
    // a fake async api call like which sends
    // 20 more records in 1.5 secs

    const data = await getRelevance(page);
    const links = processData(data);

    setTimeout(() => {
      setItems((itemState) => ({ ...itemState, ...links }));
      setPage((itemPage) => itemPage + 1);
    }, 500);
  };

  if (loading) {
    return (
      <div
        style={{
          width: '100%',
          height: '50vh',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          flexDirection: 'column',
        }}
      >
        <Loading />
      </div>
    );
  }

  return (
    <MainContainer width="90%">
      <Relevance items={items} fetchMoreData={fetchMoreData} page={page} allPage={allPage} />
    </MainContainer>
  );
}

export default Objects;
