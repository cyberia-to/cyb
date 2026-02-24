import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ActionBar, BandwidthBar, Input, MainContainer } from 'src/components';
import { CHAIN_ID } from 'src/constants/config';
import useGetSlots from 'src/containers/mint/useGetSlots';
import { useBackend } from 'src/contexts/backend/backend';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { useAdviser } from 'src/features/adviser/context';
import { useCyberlinkWithWaitAndAdviser } from 'src/features/cyberlinks/hooks/useCyberlink';
import { selectCurrentPassport } from 'src/features/passport/passports.redux';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import KeywordButton from '../landing/components/KeywordButton/KeywordButton';
import TitleText from '../landing/components/TitleText/TitleText';
import styles from './Learn.module.scss';

const learningListConfig = [
  {
    title: 'upgrade',
    text: 'your intelligence to superintelligence',
  },
  {
    title: 'spread',
    text: 'your content cheaper',
  },
  {
    title: 'upload',
    text: (
      <>
        your brain into eternity, <br /> and more
      </>
    ),
  },
];

function Learn() {
  const [ask, setAsk] = useState('');
  const [answer, setAnswer] = useState('');

  const {
    execute: cyberlink,
    isReady,
    isLoading: loading,
  } = useCyberlinkWithWaitAndAdviser({
    from: ask,
    to: answer,
    callback: ({ toCid }) => {
      navigate(routes.ipfs.getLink(toCid));
    },
  });

  const { isIpfsInitialized } = useBackend();

  const address = useCurrentAddress();

  const navigate = useNavigate();

  const citizenship = useAppSelector(selectCurrentPassport);
  const noPassport = CHAIN_ID === Networks.BOSTROM && !citizenship;

  const { balancesResource } = useGetSlots(address);
  const noEnergy = !balancesResource.millivolt || !balancesResource.milliampere;

  const { setAdviser } = useAdviser();

  useEffect(() => {
    let content;
    let adviserColor: keyof typeof AdviserColors = 'blue';

    if (noPassport) {
      content = (
        <div>
          moon <Link to={routes.portal.path}>citizenship</Link> unlocks all features and takes 3
          minutes
        </div>
      );
    } else if (noEnergy) {
      content = (
        <div>
          to cyberlink and learn you need energy <br />
          amperes, 💡 and volts, ⚡️ can be produced by freezing hydrogen H in{' '}
          <Link to={routes.hfr.path}>HFR</Link>
        </div>
      );
      adviserColor = 'red';
    } else {
      content = 'create cyberlink';
    }

    setAdviser(content, adviserColor);
  }, [noPassport, noEnergy, setAdviser]);

  return (
    <MainContainer>
      <div className={styles.wrapper}>
        <div className={styles.info}>
          <h3 className={styles.titleText}>linking is fundamental to learning</h3>
          <p>
            learning can be squeezed into one simple act: creating a link <br />
            when you link two{' '}
            <Link to={routes.search.getLink('particle')}>pieces of information</Link> then{' '}
            <Link to={routes.search.getLink('search is trivial')}>search is trivial</Link>
            <br />
            the more links you learned, the{' '}
            <Link to={routes.search.getLink('negentropy')}>smarter you are</Link>
          </p>
        </div>

        <ul className={styles.list}>
          {learningListConfig.map(({ title, text }) => {
            return (
              <li key={title}>
                <TitleText title={title} text={text} />
              </li>
            );
          })}
        </ul>

        <div className={styles.linksExample}>
          <h3 className={styles.titleText}>here are some cool links</h3>

          <ul>
            {[
              ['health', 'meditation'],
              ['music', 'joy'],
              ['cyber', 'freedom'],
              ['help', 'get H'],
              ['help', 'get energy'],
            ].map(([word1, word2], i) => {
              return (
                <li key={i}>
                  <KeywordButton keyword={word1} />
                  <span className={styles.tilde}>~</span>
                  <KeywordButton keyword={word2} />
                </li>
              );
            })}
          </ul>
        </div>

        <section className={styles.learn}>
          <h3 className={styles.titleText}>now let's learn something new</h3>

          <div className={styles.inputs}>
            <Input
              value={ask}
              disabled={noEnergy || loading}
              placeholder="Ask"
              color="pink"
              onChange={(e) => setAsk(e.target.value)}
            />
            <span className={styles.tilde}>~</span>
            <Input
              value={answer}
              disabled={noEnergy || loading}
              placeholder="Answer"
              color="pink"
              onChange={(e) => setAnswer(e.target.value)}
            />
          </div>
        </section>

        <div className={styles.energy}>
          <BandwidthBar tooltipPlacement="top" />

          <div>
            <span>{balancesResource.millivolt} ⚡️</span>
            <span>{balancesResource.milliampere} 💡</span>
          </div>
        </div>

        <ActionBar
          button={{
            disabled: !ask || !answer || !isReady || noEnergy,
            pending: loading,
            onClick: cyberlink,
            text: isIpfsInitialized ? 'cyberlink' : 'ipfs is preparing...',
          }}
        />
      </div>
    </MainContainer>
  );
}

export default Learn;
