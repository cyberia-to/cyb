import { Fragment, useEffect, useMemo } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import Loader2 from 'src/components/ui/Loader2';
import { useAdviser } from 'src/features/adviser/context';
import DateTitle from 'src/features/sense/ui/SenseViewer/Messages/DateTitle/DateTitle';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { Display, NoItems } from '../../../../../../components';
import useGetLog from '../../hooks/useGetLog';
import styles from './feeds.module.scss';
import LogItemContent from './ui/LogItemContent';
import { mapLogData, reduceByDate } from './utils';

function FeedsTab() {
  const { address, addRefetch } = useRobotContext();
  const { setAdviser } = useAdviser();
  const { data, fetchNextPage, hasNextPage, refetch, error, isInitialLoading } = useGetLog(address);

  useEffect(() => {
    if (error) {
      setAdviser(error.message, 'red');
    } else {
      setAdviser(
        <>
          free unlimited public feed made by neuron <br />
          we accepting all type of content
        </>
      );
    }

    return () => {
      // maybe remove
      setAdviser(null);
    };
  }, [setAdviser, error]);

  const logRows = useMemo(() => {
    return reduceByDate(mapLogData(data));
  }, [data]);

  useEffect(() => {
    addRefetch(refetch);
  }, [addRefetch, refetch]);

  // eslint-disable-next-line @typescript-eslint/no-empty-function
  let content;

  if (isInitialLoading) {
    content = <Loader2 />;
  } else if (error) {
    content = null;
  } else if (data.length) {
    content = (
      <InfiniteScroll
        dataLength={Object.keys(logRows).length}
        hasMore={Boolean(hasNextPage)}
        next={fetchNextPage}
        loader={<Loader2 />}
        className={styles.containerLogRows}
      >
        {logRows.map(([date, items]) => {
          return (
            <Fragment key={date}>
              {items.map((itemLog) => {
                return <LogItemContent key={itemLog.txhash} logItem={itemLog} />;
              })}
              <DateTitle date={new Date(date)} />
            </Fragment>
          );
        })}
      </InfiniteScroll>
    );
  } else {
    content = <NoItems text="No feeds" />;
  }

  return <Display>{content}</Display>;
}

export default FeedsTab;
