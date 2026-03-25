import { useEffect, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { ActionBar, Button, Tabs } from 'src/components';
import { setFocus } from 'src/containers/application/Header/Commander/commander.redux';
// import CyberlinksGraphContainer from 'src/features/cyberlinks/CyberlinksGraph/CyberlinksGraphContainer';
import { Stars } from 'src/containers/portal/components';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useAppDispatch } from 'src/redux/hooks';
import { routes } from 'src/routes';
import KeywordButton from './components/KeywordButton/KeywordButton';
import styles from './OracleLanding.module.scss';

import Stats from './Stats/Stats';
// import graphDataPrepared from './graphDataPrepared.json';
import { TitleType } from './type';

const mapTitleTypeToTitle = {
  [TitleType.search]: 'search',
  [TitleType.learning]: 'learn',
  [TitleType.ai]: 'ask',
};

const listConfig = {
  [TitleType.search]: {
    title: 'instantly and censorfree',
    text: (
      <>
        <strong>find</strong> and <strong>deliver</strong> content
      </>
    ),
    description: (
      <>
        decentralized <Link to={routes.oracle.ask.getLink('ipfs')}>ipfs</Link> search
      </>
    ),
  },
  [TitleType.learning]: {
    title: 'empower everyone',
    text: (
      <>
        <Link to={routes.oracle.learn.path}>learn</Link> yourself
      </>
    ),
    description: (
      <>
        decentralized learning as simple as creating a <strong>link</strong>
      </>
    ),
  },
  [TitleType.ai]: {
    title: 'decentralized ai is alive',
    text: (
      <>
        behold the new <Link to={routes.oracle.ask.getLink('truth')}>truth medium</Link>
      </>
    ),
    description: (
      <>
        <Link to={routes.oracle.ask.getLink('cyber')}>cyber</Link> is the protocol for unified,
        provable, collective learning
      </>
    ),
  },
};

const QUERY_KEY = 'type';

function OracleLanding() {
  const [searchParams] = useSearchParams();
  const _type = searchParams.get(QUERY_KEY);

  const [titleType, setTitleType] = useState<TitleType>(TitleType.search);

  const ref = useRef<HTMLDivElement>(null);
  const dispatch = useAppDispatch();

  useAdviserTexts({
    defaultText: 'ask your question',
  });

  useEffect(() => {
    dispatch(setFocus(true));
  }, [dispatch]);

  const { title, description, text } = listConfig[titleType];

  return (
    <div className={styles.wrapper} ref={ref}>
      <div className={styles.starsWrapper}>
        <Stars />
      </div>

      <header className={styles.header}>
        <Tabs
          options={[TitleType.search, TitleType.ai, TitleType.learning].map((type) => {
            return {
              onClick: () => {
                setTitleType(type);
                // navigate(`?${QUERY_KEY}=${mapTitleTypeToTitle[index]}`, {
                //   replace: true,
                // });
              },
              text: mapTitleTypeToTitle[type],
              key: type,
            };
          })}
          selected={titleType}
        />
      </header>

      <div className={styles.info}>
        <h2>{title}</h2>
        <h3>{text}</h3>
        <h4>{description}</h4>

        <Stats type={titleType} />
      </div>

      {/* {!isMobile && (
        <div className={styles.graphWrapper}>
        

          {isRenderGraph && (
            <CyberlinksGraphContainer
              size={graphSize}
              data={graphDataPrepared}
            />
          )}
        </div>
      )} */}

      <div className={styles.footer}>
        {[
          {
            query: 'cyber',
            author: 'bostrom1d8754xqa9245pctlfcyv8eah468neqzn3a0y0t',
          },
          {
            query: 'donut of knowledge',
            author: 'bostrom1k7nssnnvxezpp4una7lvk6j53895vadpqe6jh6',
          },
          {
            query: 'help',
            author: 'bostrom1hmkqhy8ygl6tnl5g8tc503rwrmmrkjcq3lduwj',
          },
        ].map(({ query, author }) => {
          return <KeywordButton key={query} keyword={query} author={author} />;
        })}
      </div>

      <ActionBar>
        {(() => {
          switch (titleType) {
            case TitleType.search:
              return (
                <Button link="/particles" className={styles.actionBarBtn}>
                  get high
                </Button>
              );

            case TitleType.learning:
              return (
                <Button link={routes.oracle.learn.path} className={styles.actionBarBtn}>
                  how to learn
                </Button>
              );

            case TitleType.ai:
              return (
                <Button onClick={() => dispatch(setFocus(true))} className={styles.actionBarBtn}>
                  ask something
                </Button>
              );

            default:
              return null;
          }
        })()}
      </ActionBar>
    </div>
  );
}

export default OracleLanding;
