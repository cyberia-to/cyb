import { useEffect, useMemo, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import { useSelector } from 'react-redux';
import Spark from 'src/components/search/Spark/Spark';
import { CID_TWEET } from 'src/constants/app';
import { useAdviser } from 'src/features/adviser/context';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { RootState } from 'src/redux/store';
import { Display, Dots, NoItems } from '../../components';
import useSetActiveAddress from '../../hooks/useSetActiveAddress';
import ActionBarCont from '../market/actionBarContainer';
import styles from './Taverna.module.scss';
import useGetTweets from './useGetTweets';

const LOAD_COUNT = 10;

function Taverna() {
  const { address, isOwner } = useRobotContext();
  const { defaultAccount } = useSelector((state: RootState) => state.pocket);
  const { tweets, loadingTweets } = useGetTweets(address || null);
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const [rankLink, setRankLink] = useState<string | null>();
  const [update, setUpdate] = useState(1);

  const [itemsToShow, setItemsToShow] = useState(20);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <>
        real feed <br /> no ad, no spam, no scam
      </>
    );
  }, [setAdviser]);

  useEffect(() => {
    setRankLink(null);
  }, []);

  const onClickRank = async (key: string) => {
    if (rankLink === key) {
      setRankLink(null);
    } else {
      setRankLink(key);
    }
  };

  const displayedPalettes = useMemo(
    () =>
      Object.keys(tweets)
        .sort((a, b) => {
          const x = Date.parse(tweets[a].time);
          const y = Date.parse(tweets[b].time);
          return y - x;
        })
        .slice(0, itemsToShow)
        .map((key) => {
          return (
            <Spark
              selfLinks
              key={key}
              cid={key}
              itemData={tweets[key]}
              rankSelected={rankLink === key.cid}
              handleRankClick={onClickRank}
            />
          );
        }),
    [itemsToShow, tweets, onClickRank, rankLink]
  );

  if (loadingTweets) {
    return <Dots />;
  }

  const loadMore = () => {
    setItemsToShow((i) => i + LOAD_COUNT);
  };

  return (
    <>
      <Display>
        <InfiniteScroll
          dataLength={itemsToShow}
          next={loadMore}
          // endMessage={<p>all loaded</p>}
          hasMore={Object.keys(tweets).length > itemsToShow}
          loader={<Dots />}
          className={styles.infiniteScroll}
        >
          {Object.keys(tweets).length > 0 ? displayedPalettes : <NoItems text="No feeds" />}
        </InfiniteScroll>
      </Display>

      <div
        style={{
          position: 'fixed',
          left: 0,
          zIndex: 1,
        }}
      >
        {isOwner && (
          <ActionBarCont
            addressActive={addressActive}
            keywordHash={CID_TWEET}
            updateFunc={() => setUpdate(update + 1)}
            rankLink={rankLink}
            textBtn="Tweet"
          />
        )}
      </div>
    </>
  );
}

export default Taverna;
