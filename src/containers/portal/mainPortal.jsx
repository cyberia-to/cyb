import { useEffect, useState } from 'react';
import { connect } from 'react-redux';
import { useNavigate } from 'react-router-dom';
import { useQueryClient } from 'src/contexts/queryClient';
import { MainContainer, Stars } from './components';
import { activePassport } from './utils';
import Release from './release';
import PortalGift from './gift';
import styles from './styles.scss';
// const rocketSpacePussy = require('../../image/rocket.svg');
import RocketSpacePussy from './RocketSpacePussy';
import { useAdviser } from 'src/features/adviser/context';
import { Dots } from 'src/components/ui/Dots';
import { Display } from 'src/components';
import { displayValue } from '@tanstack/react-query-devtools/build/lib/utils';

const spacePussy = require('../../image/space-pussy.svg');
const portalPussyEnter = require('../../sounds/portalPussyEnter.mp3');

const audioBtnObg = new Audio(portalPussyEnter);

const playAudioClick = () => {
  audioBtnObg.play();
};

const STAGE_LOADING = 0;
const STAGE_INIT = 1;
const STAGE_PROVE = 2;
const STAGE_RELEASE = 3;

const getActiveAddress = (address) => {
  const { account } = address;
  let addressPocket = null;
  if (
    account !== null &&
    Object.prototype.hasOwnProperty.call(account, 'cyber')
  ) {
    const { keys, bech32 } = account.cyber;
    addressPocket = {
      bech32,
      keys,
    };
  }
  return addressPocket;
};

const scaleInitValue = 0.9;

function MainPartal({ defaultAccount }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [stagePortal, setStagePortal] = useState(STAGE_LOADING);
  const [scale, setScale] = useState(scaleInitValue);

  useEffect(() => {
    const getPasport = async () => {
      if (stagePortal === STAGE_LOADING) {
        setStagePortal(STAGE_LOADING);
        if (queryClient) {
          const addressActive = getActiveAddress(defaultAccount);
          if (addressActive !== null) {
            const responseActivePassport = await activePassport(
              queryClient,
              addressActive.bech32
            );
            if (responseActivePassport !== null) {
              const { addresses } = responseActivePassport.extension;
              if (addresses !== null) {
                setStagePortal(STAGE_INIT);
              } else {
                setStagePortal(STAGE_PROVE);
              }
            } else {
              setStagePortal(STAGE_INIT);
            }
          } else {
            setStagePortal(STAGE_INIT);
          }
        } else {
          setStagePortal(STAGE_LOADING);
        }
      }
    };
    getPasport();
  }, [queryClient, defaultAccount, stagePortal]);
  console.debug('stagePortal', stagePortal);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    if (!(scale === scaleInitValue)) {
      return;
    }

    setAdviser(
      <>
        <blockquote>
          The measure of intelligence is ability to change.
        </blockquote>
        <br />
        <i>Albert Einstein</i>
      </>
    );
  }, [setAdviser, scale]);

  const onClickSpacePussy = () => {
    setScale(6);
    setTimeout(() => {
      navigate('/citizenship');
      // setScale(1);
    }, 2705);
    playAudioClick();
  };

  if (stagePortal === STAGE_LOADING) {
    return (
      <div
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <Dots />
      </div>
    );
  }

  if (stagePortal === STAGE_INIT) {
    return (
      <MainContainer>
        <Stars />

        <div
          style={{
            position: 'fixed',
            zIndex: '3',
            left: '50%',
            marginRight: '-50%',
            transform: `translate(-50%, 40px) scale(${scale})`,
            transition: 'all 2.7s cubic-bezier(0.67, 0.01, 0.37, 1.01) 0s',
          }}
        >
          <img style={{ width: '100%' }} src={spacePussy} alt="spacePussy" />
          <button
            type="button"
            onClick={() => onClickSpacePussy()}
            className={styles.buttonSpacePussy}
          >
            <div className={styles.textSpacePussy}>cyberverse</div>
            {/* <img
              className={styles.arrowSpacePussy}
              src={rocketSpacePussy}
              alt="arrowSpacePussy"
            /> */}
            <RocketSpacePussy />
          </button>
        </div>
      </MainContainer>
    );
  }

  if (stagePortal === STAGE_PROVE) {
    return <PortalGift />;
  }

  if (stagePortal === STAGE_RELEASE) {
    return <Release />;
  }

  return null;
}

const mapStateToProps = (store) => {
  return {
    defaultAccount: store.pocket.defaultAccount,
  };
};

export default connect(mapStateToProps)(MainPartal);
