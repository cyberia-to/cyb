/* eslint-disable no-nested-ternary */
import { useCallback, useEffect, useRef, useState } from 'react';
import useMediaQuery from 'src/hooks/useMediaQuery';
import { formatCurrency, formatNumber, timeSince } from '../../../../utils/utils';
import {
  useAccountCount,
  useContractsCount,
  useGetGraphStats,
  useGetNegentropy,
  useGetPortalStats,
  useGetTotalCap,
  useGetValidatorsBonded,
} from '../../hooks';
import slideData from './slideData';
import styles from './styles.scss';

const PREFIXES = [
  {
    prefix: 'T',
    power: 1024 * 10 ** 9,
  },
  {
    prefix: 'G',
    power: 1024 * 10 ** 6,
  },
  {
    prefix: 'M',
    power: 1024 * 10 ** 3,
  },
  {
    prefix: 'K',
    power: 1024,
  },
];

export function TypingText({ content, delay = 30 }) {
  const [displayed, updateDisplay] = useState('');
  const animIDRef = useRef(null);

  useEffect(() => {
    updateDisplay(content.charAt(0));
    animIDRef.current = setInterval(() => {
      updateDisplay((prevText) => {
        if (content.length <= prevText.length) {
          clearInterval(animIDRef.current);
          return prevText;
        }
        return prevText.concat(content.charAt(prevText.length));
      });
    }, delay);
    return () => {
      updateDisplay('');
      clearInterval(animIDRef.current);
    };
  }, [content, delay]);

  return displayed;
}

function DeltaValue({ change }) {
  if (parseFloat(change.amount) > 0) {
    return (
      <div
        style={{
          color: parseFloat(change.amount) >= 0 ? '#76FF03' : '#FF0000',
          fontSize: 20,
          textShadow:
            parseFloat(change.amount) >= 0 ? '0px 4px 10px #76FF03' : '0px 4px 10px #FF0000',
        }}
      >
        {parseFloat(change.amount) !== 0 ? (parseFloat(change.amount) > 0 ? '+' : '') : ''}
        {change.amount} in {timeSince(change.time)}
      </div>
    );
  }

  return null;
}

const delay = 4000;

function PlayBanerContent() {
  const mediaQuery = useMediaQuery('(min-width: 768px)');
  const dataAccountCount = useAccountCount();
  const dataGetPortalStats = useGetPortalStats();
  const dataGetNegentropy = useGetNegentropy();
  const dataGetValidatorsBonded = useGetValidatorsBonded();
  const dataContractsCount = useContractsCount();
  const dataGetGraphStats = useGetGraphStats();
  const dataGetTotalCap = useGetTotalCap();
  const slideDataRef = useRef(slideData);
  const timeoutRef = useRef(null);

  const [index, setIndex] = useState(0);

  useEffect(() => {
    slideDataRef.current = {
      ...slideDataRef.current,
      fastGrowing: {
        ...slideDataRef.current.fastGrowing,
        amount: formatNumber(dataGetTotalCap.capData.currentCap),
        change: {
          time: dataGetTotalCap.capData.change.time,
          amount: `${dataGetTotalCap.capData.change.amount}%`,
        },
      },
      futureOf: {
        ...slideDataRef.current.futureOf,
        amount: formatNumber(dataGetTotalCap.priceData.price),
        change: {
          time: dataGetTotalCap.priceData.change.time,
          amount: `${dataGetTotalCap.priceData.change.amount}%`,
        },
      },
    };
  }, [dataGetTotalCap]);

  useEffect(() => {
    if (dataGetNegentropy.status === 'success' && dataGetNegentropy.data) {
      slideDataRef.current = {
        ...slideDataRef.current,
        realDistributed: {
          ...slideDataRef.current.realDistributed,
          amount: formatNumber(dataGetNegentropy.data.negentropy),
          change: {
            time: dataGetNegentropy.changeTimeAmount.time,
            amount: formatNumber(dataGetNegentropy.changeTimeAmount.amount),
          },
        },
      };
    }
  }, [dataGetNegentropy]);

  useEffect(() => {
    if (dataAccountCount.data) {
      slideDataRef.current = {
        ...slideDataRef.current,
        everGrowing: {
          ...slideDataRef.current.everGrowing,
          amount: formatNumber(dataAccountCount.data.accountCount),
          change: {
            time: dataAccountCount.changeTimeAmount.time,
            amount: formatNumber(dataAccountCount.changeTimeAmount.amount),
          },
        },
      };
    }
  }, [dataAccountCount]);

  useEffect(() => {
    if (dataContractsCount.data) {
      slideDataRef.current = {
        ...slideDataRef.current,
        autonomousSemantic: {
          ...slideDataRef.current.autonomousSemantic,
          amount: formatNumber(dataContractsCount.data.contractsCount),
          change: {
            time: dataContractsCount.changeTimeAmount.time,
            amount: formatNumber(dataContractsCount.changeTimeAmount.amount),
          },
        },
      };
    }
  }, [dataContractsCount]);

  useEffect(() => {
    if (dataGetPortalStats.status === 'success' && dataGetPortalStats.data) {
      slideDataRef.current = {
        ...slideDataRef.current,
        superintelligentMoon: {
          ...slideDataRef.current.superintelligentMoon,
          amount: formatNumber(dataGetPortalStats.data.citizens),
          change: {
            time: dataGetPortalStats.changeTimeAmount.time,
            amount: formatNumber(dataGetPortalStats.changeTimeAmount.citizensAmount),
          },
        },
        provable: {
          ...slideDataRef.current.provable,
          amount: `${formatNumber(dataGetPortalStats.data.procentClaim)} %`,
          change: {
            time: dataGetPortalStats.changeTimeAmount.time,
            amount: `${formatNumber(dataGetPortalStats.changeTimeAmount.procentClaimAmount)} %`,
          },
        },
      };
    }
  }, [dataGetPortalStats]);

  useEffect(() => {
    if (dataGetValidatorsBonded.status === 'success' && dataGetValidatorsBonded.data) {
      slideDataRef.current = {
        ...slideDataRef.current,
        biggestUseful: {
          ...slideDataRef.current.biggestUseful,
          amount: formatNumber(dataGetValidatorsBonded.data.validators),
          change: {
            time: dataGetValidatorsBonded.changeTimeAmount.time,
            amount: formatNumber(dataGetValidatorsBonded.changeTimeAmount.amount),
          },
        },
      };
    }
  }, [dataGetValidatorsBonded]);

  useEffect(() => {
    if (dataGetGraphStats.status === 'success' && dataGetGraphStats.data) {
      const { cyberlinks, particles, beta, bits } = dataGetGraphStats.data;
      const { changeTimeAmount } = dataGetGraphStats;

      slideDataRef.current = {
        ...slideDataRef.current,
        collaborativeNeural: {
          ...slideDataRef.current.collaborativeNeural,
          amount: formatNumber(cyberlinks),
          change: {
            amount: formatNumber(changeTimeAmount.cyberlinks),
            time: changeTimeAmount.time,
          },
        },
        unstoppablePublic: {
          ...slideDataRef.current.unstoppablePublic,
          amount: formatCurrency(bits, 'B', 0, PREFIXES),
          change: {
            amount: formatCurrency(changeTimeAmount.bits, 'B', 0, PREFIXES),
            time: changeTimeAmount.time,
          },
        },
        nextGen: {
          ...slideDataRef.current.nextGen,
          amount: formatNumber(particles),
          change: {
            amount: formatNumber(changeTimeAmount.particles),
            time: changeTimeAmount.time,
          },
        },
        permissionlessOnchain: {
          ...slideDataRef.current.permissionlessOnchain,
          amount: formatNumber(beta),
          change: {
            amount: formatNumber(changeTimeAmount.beta),
            time: changeTimeAmount.time,
          },
        },
      };
    }
  }, [dataGetGraphStats]);

  function resetTimeout() {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
  }

  useEffect(() => {
    resetTimeout();
    if (index === Object.values(slideData).length - 1) {
      resetTimeout();
    } else {
      timeoutRef.current = setTimeout(
        () =>
          setIndex((prevIndex) =>
            prevIndex === Object.values(slideData).length - 1
              ? Object.values(slideData).length - 1
              : prevIndex + 1
          ),
        delay
      );
    }

    return () => {
      resetTimeout();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [index, resetTimeout]);

  const restartSlide = useCallback(() => {
    if (index === Object.values(slideData).length - 1) {
      resetTimeout();
      setIndex(0);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [index, resetTimeout]);

  const slideDataState = slideDataRef.current;
  return (
    // eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions
    <div className={styles.gatadienContainer} onClick={() => restartSlide()}>
      <div
        className={styles.containerTitle}
        style={{
          flexDirection: mediaQuery ? 'row' : 'column',
          alignItems: mediaQuery ? 'center' : 'flex-start',
        }}
      >
        <div>
          <TypingText content={Object.values(slideDataState)[index].title} delay={40} />
        </div>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'flex-end',
            width: mediaQuery ? 'unset' : '100%',
          }}
        >
          <DeltaValue change={Object.values(slideDataState)[index].change} />
        </div>
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          padding: '15px 0px',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            flexDirection: mediaQuery ? 'row' : 'column',
            color: '#FFD900',
            textShadow: '0px 0px 10px #FCC539',
            fontSize: mediaQuery ? 35 : 25,
            gap: 10,
          }}
        >
          <div>{Object.values(slideDataState)[index].amount}</div>
          <div> {Object.values(slideDataState)[index].keyAmount}</div>
        </div>
      </div>
    </div>
  );
}

export default PlayBanerContent;
