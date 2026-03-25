import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSelector } from 'react-redux';
import portalAmbient from 'sounds/portalAmbient112.mp3';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useDevice } from 'src/contexts/device';
import { useAdviser } from 'src/features/adviser/context';
import { RootState } from 'src/redux/store';
import { Nullable } from 'src/types';
import Carousel from '../../../components/Tabs/Carousel/CarouselOld/CarouselOld';
import useSetActiveAddress from '../../../hooks/useSetActiveAddress';
import { AboutGift, CurrentGift, MainContainer, MoonAnimation, Stars } from '../components';
import ReleaseStatus from '../components/ReleaseStatus';
import useCheckGift from '../hook/useCheckGift';
import useCheckRelease from '../hook/useCheckRelease';
import useGetStatGift from '../hook/useGetStatGift';
import usePingTxs from '../hook/usePingTxs';
import PasportCitizenship from '../pasport';
import { filterByOwner } from '../release';
import ActionBarRelease from '../release/ActionBarRelease';
import { CurrentRelease, ReadyRelease } from '../release/type';
import { AMOUNT_ALL_STAGE, NEW_RELEASE, useGetActivePassport } from '../utils';
import ActionBarPortalGift from './ActionBarPortalGift';
import Info from './Info';
import STEP_INFO from './utils';

const portalAmbientObg = new Audio(portalAmbient);

const playPortalAmbient = () => {
  portalAmbientObg.loop = true;
  portalAmbientObg.play();
};

const stopPortalAmbient = () => {
  portalAmbientObg.loop = false;
  portalAmbientObg.pause();
  portalAmbientObg.currentTime = 0;
};

const STEP_GIFT_INFO = 1;
const STEP_PROVE_ADD = 2;
const STEP_CLAIME = 3;
const STEP_RELEASE = 4;

const itemsStep = [
  {
    title: 'gift',
    step: STEP_GIFT_INFO,
  },
  {
    title: 'prove address',
    step: STEP_PROVE_ADD,
  },
  {
    title: 'claim',
    step: STEP_CLAIME,
  },
  {
    title: 'release',
    step: STEP_RELEASE,
  },
];

function PortalGift() {
  const { isMobile: mobile } = useDevice();
  const [appStep, setStepApp] = useState(STEP_INFO.STATE_INIT);
  const { defaultAccount } = useSelector((store: RootState) => store.pocket);
  const { addressActive } = useSetActiveAddress(defaultAccount, false);
  const { txHash, updateFunc, updateTxHash } = usePingTxs();
  const { citizenship, loading } = useGetActivePassport(addressActive, updateFunc);
  const { totalGift, totalGiftClaimed, loadingGift, giftData, setLoadingGift } = useCheckGift(
    citizenship,
    addressActive,
    updateFunc
  );
  const { currentBonus, claimStat, currentStage, progressClaim } = useGetStatGift();
  const {
    totalRelease,
    totalReadyRelease,
    totalBalanceClaimAmount,
    loadingRelease,
    alreadyClaimed,
  } = useCheckRelease(totalGift, addressActive, loadingGift, updateFunc, currentStage);

  const [selectedAddress, setSelectedAddress] = useState<null | string>(null);
  const [currentGift, setCurrentGift] = useState(null);
  const [isClaimed, setIsClaimed] = useState(null);
  const [error, setError] = useState<string>();

  const [isRelease, setIsRelease] = useState(false);
  const [currentRelease, setCurrentRelease] = useState<Nullable<CurrentRelease[]>>(null);
  const [readyRelease, setReadyRelease] = useState<Nullable<ReadyRelease>>(null);

  useEffect(() => {
    playPortalAmbient();

    return () => {
      stopPortalAmbient();
    };
  }, []);

  useEffect(() => {
    if (txHash && txHash.status !== 'pending') {
      if (
        appStep === STEP_INFO.STATE_PROVE_IN_PROCESS &&
        txHash.status === 'confirmed' &&
        !loading &&
        citizenship
      ) {
        setStepApp(STEP_INFO.STATE_CLAIME);
      }

      if (appStep === STEP_INFO.STATE_PROVE_IN_PROCESS && txHash.status === 'error') {
        setStepApp(STEP_INFO.STATE_PROVE);
      }

      if (appStep === STEP_INFO.STATE_CLAIM_IN_PROCESS && txHash.status === 'confirmed') {
        setStepApp(STEP_INFO.STATE_RELEASE_INIT);
      }

      if (appStep === STEP_INFO.STATE_CLAIM_IN_PROCESS && txHash.status === 'error') {
        setStepApp(STEP_INFO.STATE_CLAIME);
      }
      setTimeout(() => updateTxHash(null), 35000);
    }
  }, [txHash, appStep, loading, citizenship, updateTxHash]);

  useEffect(() => {
    if (appStep === STEP_INFO.STATE_INIT && selectedAddress === null && citizenship) {
      setSelectedAddress(citizenship.owner);
    }
  }, [selectedAddress, appStep, citizenship]);

  useEffect(() => {
    if (Math.floor(appStep) === STEP_INFO.STATE_INIT) {
      if (!loading) {
        if (!citizenship) {
          setStepApp(STEP_INFO.STATE_INIT_NULL);
        } else if (!loadingGift) {
          if (isClaimed !== null && !isClaimed) {
            setStepApp(STEP_INFO.STATE_INIT_CLAIM);
          } else {
            setStepApp(STEP_INFO.STATE_INIT_PROVE);
          }
        }
      }
    }

    if (appStep === STEP_INFO.STATE_PROVE) {
      setStepApp(STEP_INFO.STATE_PROVE_CONNECT);
    }

    if (Math.floor(appStep) === STEP_INFO.STATE_CLAIME) {
      if (!loadingGift) {
        if (!citizenship) {
          setStepApp(STEP_INFO.STATE_CLAIME_TO_PROVE);
        } else if (totalGift === null && selectedAddress && selectedAddress.match(PATTERN_CYBER)) {
          setStepApp(STEP_INFO.STATE_GIFT_NULL_ALL);
        } else if (isClaimed === null) {
          setStepApp(STEP_INFO.STATE_CLAIME_TO_PROVE);
        } else if (!isClaimed) {
          setStepApp(STEP_INFO.STATE_CLAIME);
        } else if (isClaimed) {
          setStepApp(STEP_INFO.STATE_RELEASE);
        }
      }
    }

    if (Math.floor(appStep) === STEP_RELEASE && !loadingRelease) {
      if (currentRelease !== null) {
        if (isRelease) {
          setStepApp(STEP_INFO.STATE_RELEASE_INIT);
        } else {
          setStepApp(STEP_INFO.STATE_RELEASE_ALL);
        }
      } else {
        setStepApp(STEP_INFO.STATE_RELEASE_NULL);
      }
    }
  }, [
    appStep,
    citizenship,
    isClaimed,
    totalGift,
    loadingGift,
    selectedAddress,
    loading,
    loadingRelease,
    isRelease,
    currentRelease,
  ]);

  const initState = () => {
    setReadyRelease(null);
    setIsRelease(false);
  };

  useEffect(() => {
    if (selectedAddress && totalRelease && !loadingRelease) {
      initState();
      if (Object.hasOwn(totalRelease, selectedAddress)) {
        const {
          balanceClaim: readyReleaseAddrr,
          stage,
          addressOwner,
        } = totalRelease[selectedAddress];
        setCurrentRelease([totalRelease[selectedAddress]]);
        setIsRelease(stage < currentStage);
        setReadyRelease({
          address: selectedAddress,
          amount: readyReleaseAddrr,
          addressOwner,
        });
      } else if (selectedAddress?.match(PATTERN_CYBER)) {
        if (totalReadyRelease) {
          const filterData = filterByOwner(totalReadyRelease, selectedAddress);
          setCurrentRelease(filterData);
          setIsRelease(filterData.length > 0);
          setReadyRelease({
            address: selectedAddress,
            amount: totalBalanceClaimAmount,
            addressOwner: selectedAddress,
          });
        } else {
          setCurrentRelease([]);
          setIsRelease(false);
          setReadyRelease({
            address: selectedAddress,
            amount: totalBalanceClaimAmount,
            addressOwner: selectedAddress,
          });
        }
      } else {
        setCurrentRelease(null);
      }
    } else {
      setCurrentRelease(null);
    }
  }, [
    selectedAddress,
    totalRelease,
    totalBalanceClaimAmount,
    totalReadyRelease,
    loadingRelease,
    currentStage,
    initState,
  ]);

  useEffect(() => {
    if (!loadingGift) {
      if (totalGift !== null) {
        const tempGift = [];
        Object.keys(totalGift).forEach((key) => {
          if (!totalGift[key].isClaimed) {
            tempGift.push({ ...totalGift[key] });
          }
        });

        if (Object.keys(tempGift).length > 0) {
          setCurrentGift(tempGift);
        }
      } else {
        setCurrentGift(null);
      }
    }
  }, [loadingGift, totalGift]);

  useEffect(() => {
    if (selectedAddress !== null && totalGift !== null) {
      if (Object.hasOwn(totalGift, selectedAddress)) {
        if (Object.hasOwn(totalGift[selectedAddress], 'isClaimed')) {
          const { isClaimed: isClaimedAddress } = totalGift[selectedAddress];
          setIsClaimed(isClaimedAddress);
        }
      } else if (selectedAddress?.match(PATTERN_CYBER) && totalGift !== null) {
        const tempGift = [];
        Object.keys(totalGift).forEach((key) => {
          if (!totalGift[key].isClaimed) {
            tempGift.push({ ...totalGift[key] });
          }
        });

        if (Object.keys(tempGift).length > 0) {
          setIsClaimed(false);
        }

        if (Object.keys(tempGift).length === 0) {
          setIsClaimed(true);
        }
      } else {
        setIsClaimed(null);
      }
    } else {
      setIsClaimed(null);
    }
  }, [selectedAddress, totalGift]);

  const useSelectedGiftData = useMemo(() => {
    try {
      if (selectedAddress !== null) {
        if (selectedAddress.match(PATTERN_CYBER) && totalGiftClaimed !== null) {
          const { amount } = totalGiftClaimed;
          if (amount && amount > 0) {
            return { address: selectedAddress, ...totalGiftClaimed };
          }
        }

        if (totalGift?.[selectedAddress]?.isClaimed) {
          return totalGift[selectedAddress];
        }

        if (totalGift !== null && Object.hasOwn(totalGift, selectedAddress)) {
          return totalGift[selectedAddress];
        }
      }

      return null;
    } catch (error) {
      console.log('error', error);
      return null;
    }
  }, [selectedAddress, totalGift, totalGiftClaimed]);

  const useDisableNext = useMemo(() => {
    if (citizenship) {
      return false;
    }
    return true;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [citizenship]);

  const useSetActiveItem = useMemo(() => {
    if (
      txHash !== null &&
      appStep === STEP_INFO.STATE_PROVE_IN_PROCESS &&
      txHash.status === 'confirmed' &&
      !loading &&
      citizenship
    ) {
      const { addresses } = citizenship.extension;
      if (addresses && addresses !== null) {
        const lastIndex = Object.keys(addresses).length - 1;
        return lastIndex + 1;
      }
    }
    return undefined;
  }, [loading, appStep, txHash, citizenship]);

  const useUnClaimedGiftData = useMemo(() => {
    if (giftData !== null && citizenship && Object.keys(giftData.unClaimed.addresses).length > 0) {
      if (currentBonus?.current) {
        giftData.unClaimed.claim = Math.floor(giftData.unClaimed.amount * currentBonus.current);
        return { ...giftData.unClaimed, address: citizenship.owner };
      }
    }
    return null;
  }, [giftData, currentBonus, citizenship]);

  const useReleasedStage = useMemo(() => {
    const statusRelease = {
      gift: 0,
      availableRelease: 0,
      released: 0,
      leftRelease: 0,
      alreadyClaimed: 0,
    };

    if (useSelectedGiftData && readyRelease && !loading && selectedAddress && addressActive) {
      const { bech32 } = addressActive;
      const { amount, address, addressOwner } = readyRelease;
      const { claim, address: addressGift } = useSelectedGiftData;
      if (claim && address === addressGift) {
        let released = 0;
        let claimAmount = new BigNumber(claim);

        if (selectedAddress?.match(PATTERN_CYBER)) {
          claimAmount = claimAmount.minus(alreadyClaimed);
        }

        released = new BigNumber(claimAmount).minus(amount).toNumber();
        const currentStageProcent = new BigNumber(currentStage).dividedBy(100).toNumber();

        const availableRelease = new BigNumber(claimAmount)
          .multipliedBy(currentStageProcent)
          .minus(released)
          .dp(0, BigNumber.ROUND_FLOOR)
          .toNumber();

        const availableReleaseAmount = availableRelease > 0 ? availableRelease : 0;

        statusRelease.gift = claim;
        statusRelease.leftRelease = amount;
        statusRelease.released = released;
        statusRelease.availableRelease = availableReleaseAmount;

        if (selectedAddress?.match(PATTERN_CYBER) && alreadyClaimed) {
          statusRelease.alreadyClaimed = alreadyClaimed;
        }

        if (bech32 !== addressOwner) {
          statusRelease.alreadyClaimed = amount;
          statusRelease.leftRelease = 0;
        }
      }
    }

    return statusRelease;
  }, [
    readyRelease,
    useSelectedGiftData,
    currentStage,
    selectedAddress,
    alreadyClaimed,
    loading,
    addressActive,
  ]);

  const availableRelease = useCallback(
    (isNanoLedger: boolean) => {
      if (!totalGift || !currentRelease || !addressActive) {
        return 0;
      }

      const sliceArrayRelease = currentRelease.slice(0, isNanoLedger ? 1 : currentRelease.length);

      const claimedAmount = sliceArrayRelease.reduce((sum, item) => {
        if (item.addressOwner === addressActive.bech32 && totalGift[item.address]?.claim) {
          return sum + totalGift[item.address].claim;
        }
        return sum;
      }, 0);

      const alreadyClaimed = sliceArrayRelease.reduce((sum, item) => {
        if (item.addressOwner === addressActive.bech32) {
          return sum + item.balanceClaim;
        }
        return sum;
      }, 0);

      const currentStageProcent = new BigNumber(currentStage).dividedBy(100).toNumber();

      const released = new BigNumber(claimedAmount).minus(alreadyClaimed);

      const availableRelease = new BigNumber(claimedAmount)
        .multipliedBy(currentStageProcent)
        .minus(released)
        .dp(0, BigNumber.ROUND_FLOOR)
        .toNumber();

      return availableRelease > 0 ? availableRelease : 0;
    },
    [currentRelease, totalGift, addressActive, currentStage]
  );

  const useNextRelease = useMemo(() => {
    if (currentStage < AMOUNT_ALL_STAGE && claimStat) {
      const nextTarget = new BigNumber(1).plus(currentStage).multipliedBy(NEW_RELEASE);
      return new BigNumber(nextTarget).minus(claimStat.citizensClaim).toNumber();
    }

    return 0;
  }, [currentStage, claimStat]);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    if (!appStep) {
      return;
    }

    if (error) {
      setAdviser(error, 'red');
    } else {
      setAdviser(
        <Info
          stepCurrent={appStep}
          nextRelease={useNextRelease}
          useReleasedStage={useReleasedStage}
        />
      );
    }
  }, [appStep, useReleasedStage, useNextRelease, setAdviser, error]);

  const redirectFunc = (key: 'claim' | 'prove') => {
    if (key === 'claim') {
      setStepApp(STEP_INFO.STATE_CLAIME);
    }

    if (key === 'prove') {
      setStepApp(STEP_INFO.STATE_PROVE_CONNECT);
    }
  };

  let content;

  if (Math.floor(appStep) === STEP_GIFT_INFO) {
    content = <AboutGift addressesClaimed={claimStat.citizensClaim} coefficient={currentBonus} />;
  }

  if (
    Math.floor(appStep) === STEP_PROVE_ADD ||
    Math.floor(appStep) === STEP_CLAIME ||
    Math.floor(appStep) === STEP_RELEASE
  ) {
    content = (
      <>
        <PasportCitizenship
          txHash={txHash}
          citizenship={citizenship}
          updateFunc={setSelectedAddress}
          setActiveItem={useSetActiveItem}
          totalGift={totalGift}
        />

        {useUnClaimedGiftData !== null &&
          selectedAddress !== null &&
          selectedAddress.match(PATTERN_CYBER) && (
            <CurrentGift
              title="Unclaimed"
              valueTextResult="unclaimed"
              initStateCard={false}
              selectedAddress={selectedAddress}
              currentGift={useUnClaimedGiftData}
            />
          )}

        {useSelectedGiftData !== null && Math.floor(appStep) !== STEP_RELEASE && (
          <CurrentGift
            title="Claimed"
            valueTextResult="claimed"
            selectedAddress={selectedAddress}
            currentGift={useSelectedGiftData}
            currentBonus={currentBonus}
          />
        )}

        {Math.floor(appStep) === STEP_RELEASE && (
          <ReleaseStatus
            data={useReleasedStage}
            progress={progressClaim}
            amountGiftValue={useReleasedStage.gift}
            nextRelease={useNextRelease}
          />
        )}
      </>
    );
  }

  return (
    <>
      <MainContainer>
        <Stars />
        {!mobile && <MoonAnimation />}

        <Carousel
          slides={itemsStep}
          activeStep={Math.floor(appStep)}
          setStep={setStepApp}
          disableNext={useDisableNext}
        />
        {content}
      </MainContainer>
      {Math.floor(appStep) !== STEP_RELEASE && (
        <ActionBarPortalGift
          currentBonus={currentBonus.current}
          progressClaim={progressClaim}
          addressActive={addressActive}
          citizenship={citizenship}
          updateTxHash={updateTxHash}
          isClaimed={isClaimed}
          selectedAddress={selectedAddress}
          currentGift={currentGift}
          activeStep={appStep}
          setStepApp={setStepApp}
          setLoadingGift={setLoadingGift}
          loadingGift={loadingGift}
        />
      )}
      {Math.floor(appStep) === STEP_RELEASE && (
        <ActionBarRelease
          addressActive={addressActive}
          updateTxHash={updateTxHash}
          selectedAddress={selectedAddress}
          txHash={txHash}
          currentRelease={currentRelease}
          totalGift={totalGift}
          callback={(err) => {
            setError(err);
          }}
          isRelease={isRelease}
          totalRelease={totalRelease}
          loadingRelease={loadingRelease}
          redirectFunc={redirectFunc}
          availableRelease={availableRelease}
        />
      )}
    </>
  );
}

export default PortalGift;
