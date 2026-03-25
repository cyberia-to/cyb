import { Pane } from '@cybercongress/gravity';
import InfiniteScroll from 'react-infinite-scroll-component';
import { Dots, NoItems, Rank } from '../../components';
import ContentItem from '../../components/ContentItem/contentItem';
import { exponentialToDecimal } from '../../utils/utils';

function SearchTokenInfo({
  data,
  mobile,
  selectedTokens,
  onClickRank,
  fetchMoreData,
  page,
  allPage,
  parentId,
}) {
  try {
    const apps = [];
    if (Object.keys(data).length > 0) {
      apps.push(
        Object.keys(data).map((key, i) => {
          return (
            <Pane
              position="relative"
              className="hover-rank"
              display="flex"
              alignItems="center"
              marginBottom="10px"
              key={`${key}_${i}`}
            >
              {!mobile && (
                <Pane className="time-discussion rank-contentItem" position="absolute">
                  <Rank
                    hash={key}
                    rank={exponentialToDecimal(parseFloat(data[key].rank).toPrecision(3))}
                    grade={data[key].grade}
                    onClick={() => onClickRank(key)}
                  />
                </Pane>
              )}
              <ContentItem cid={key} item={data[key]} className="contentItem" parent={parentId} />
            </Pane>
          );
        })
      );
    }
    return (
      <Pane width="90%" marginX="auto" marginY={0} display="flex" flexDirection="column">
        <div className="container-contentItem" style={{ width: '100%' }}>
          <InfiniteScroll
            dataLength={Object.keys(data).length}
            next={fetchMoreData}
            hasMore={page < allPage}
            loader={
              <h4>
                Loading
                <Dots />
              </h4>
            }
            pullDownToRefresh
            pullDownToRefreshContent={
              <h3 style={{ textAlign: 'center' }}>&#8595; Pull down to refresh</h3>
            }
            releaseToRefreshContent={
              <h3 style={{ textAlign: 'center' }}>&#8593; Release to refresh</h3>
            }
            refreshFunction={fetchMoreData}
          >
            {Object.keys(data).length > 0 ? (
              apps
            ) : (
              <NoItems text={`No information about ${selectedTokens}`} />
            )}
          </InfiniteScroll>
        </div>
      </Pane>
    );
  } catch (error) {
    console.log(error);
    return <div>oops...</div>;
  }
}

export default SearchTokenInfo;
