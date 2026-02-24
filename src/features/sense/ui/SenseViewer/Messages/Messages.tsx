import dateFormat from 'dateformat';
import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import InfiniteScroll from 'react-infinite-scroll-component';
import { LLMMessage, SenseItem } from 'src/features/sense/redux/sense.redux';
import DateTitle from './DateTitle/DateTitle';
import MessageContainer from './Message/Message.container';
import MessageComponent from './Message/MessageComponent';
import styles from './Messages.module.scss';

type Props = {
  messages: SenseItem[] | LLMMessage[];
  currentChatId: string;
  // Remove 'currentChatId' if not needed
};

// lengths in days
const DEFAULT_ITEMS_LENGTH = 5;
const LOAD_MORE_ITEMS_LENGTH = 3;

function Messages({ messages, currentChatId }: Props) {
  const [showItemsLength, setShowItemsLength] = useState(DEFAULT_ITEMS_LENGTH);

  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setShowItemsLength(DEFAULT_ITEMS_LENGTH);
  }, []);

  useEffect(() => {
    if (!ref.current) {
      return;
    }

    ref.current.scrollTop = ref.current.scrollHeight;
  }, []);

  function setMore() {
    setShowItemsLength((showItemsLength) => showItemsLength + LOAD_MORE_ITEMS_LENGTH);
  }

  const messagesByDateAll = useMemo(() => {
    return Object.entries(
      [...messages].reverse().reduce<{
        [date: string]: SenseItem[] | LLMMessage[];
      }>((acc, senseItem) => {
        const date = dateFormat(senseItem.timestamp, 'yyyy-mm-dd');

        if (!acc[date]) {
          acc[date] = [];
        }

        acc[date].push(senseItem);

        return acc;
      }, {})
    );
  }, [messages]);

  const messagesByDate = useMemo(
    () => messagesByDateAll.slice(0, showItemsLength),
    [messagesByDateAll, showItemsLength]
  );

  return (
    // wrappers for correct scroll
    <div className={styles.wrapper}>
      <div className={styles.messages} ref={ref}>
        {currentChatId === 'llm' ? (
          <InfiniteScroll
            scrollableTarget={ref.current}
            inverse
            loader={null}
            scrollThreshold={0.9}
            className={styles.inner}
            dataLength={(messages as LLMMessage[]).length}
            hasMore={false}
            next={() => {}}
          >
            {/* {messagesByDateAll.map((message, index) => (
              <MessageComponent key={index} message={message} />
            ))} */}

            {messagesByDate.map(([date, messages]) => (
              <Fragment key={date}>
                {messages.map((senseItem) => (
                  <MessageComponent
                    key={senseItem.timestamp}
                    message={senseItem}
                    // currentChatId={currentChatId}
                  />
                ))}
                <DateTitle date={new Date(date)} />
              </Fragment>
            ))}
          </InfiniteScroll>
        ) : (
          <InfiniteScroll
            scrollableTarget={ref.current}
            inverse
            loader={null}
            scrollThreshold={0.9}
            className={styles.inner}
            dataLength={messagesByDate.length}
            hasMore={showItemsLength < messages.length}
            next={setMore}
          >
            {messagesByDate.map(([date, messages]) => (
              <Fragment key={date}>
                {messages.map((senseItem) => (
                  <MessageContainer
                    key={senseItem.transactionHash}
                    senseItem={senseItem}
                    currentChatId={currentChatId}
                  />
                ))}
                <DateTitle date={new Date(date)} />
              </Fragment>
            ))}
          </InfiniteScroll>
        )}
      </div>
    </div>
  );
}

export default Messages;
