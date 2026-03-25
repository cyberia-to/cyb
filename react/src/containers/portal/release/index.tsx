import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSelector } from 'react-redux';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useDevice } from 'src/contexts/device';
import { useAdviser } from 'src/features/adviser/context';
import { RootState } from 'src/redux/store';
import { Nullable } from 'src/types';
import useSetActiveAddress from '../../../hooks/useSetActiveAddress';
import portalAmbient from '../../../sounds/portalAmbient112.mp3';
import { CurrentGift, MainContainer, MoonAnimation, Stars } from '../components';
import ReleaseStatus from '../components/ReleaseStatus';
import useCheckGift from '../hook/useCheckGift';
import useCheckRelease, { TotalRelease } from '../hook/useCheckRelease';
import useGetStatGift from '../hook/useGetStatGift';
import usePingTxs from '../hook/usePingTxs';
import PasportCitizenship from '../pasport';
import { AMOUNT_ALL_STAGE, NEW_RELEASE, useGetActivePassport } from '../utils';
import ActionBarRelease from './ActionBarRelease';
import Info from './Info';
import { CurrentRelease, ReadyRelease } from './type';
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

export const filterByOwner = (data: TotalRelease[], ownerAddress: string) => {
  return data.filter((item) => item.addressOwner === ownerAddress);
};

const { STATE_READY_TO_RELEASE, STATE_NEXT_UNFREEZE, STATE_PROVE_ADDRESS, STATE_INIT_NULL_ACTIVE } =
  STEP_INFO;

function Release() {
  const { isMobile: mobile } = useDevice();
  const { defaultAccount } = useSelector((store: RootState) => store.pocket);
  const { txHash, updateFunc, updateTxHash } = usePingTxs();
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const [error, setError] = useState<string>();

  const { citizenship, loading: loadingCitizenship } = useGetActivePassport(addressActive);
  const { totalGift, totalGiftClaimed, giftData, loadingGift } = useCheckGift(
    citizenship,
    addressActive
  );
  const { loading, currentBonus, claimStat, currentStage, progressClaim } = useGetStatGift();
  const {
    totalRelease,
    totalReadyRelease,
    totalBalanceClaimAmount,
    loadingRelease,
    alreadyClaimed,
  } = useCheckRelease(totalGift, addressActive, loadingGift, updateFunc, currentStage);

  const [selectedAddress, setSelectedAddress] = useState<Nullable<string>>(null);
  const [isRelease, setIsRelease] = useState(false);
  const [currentRelease, setCurrentRelease] = useState<Nullable<CurrentRelease[]>>(null);
  const [readyRelease, setReadyRelease] = useState<Nullable<ReadyRelease>>(null);
  const [stateInfo, setStateInfo] = useState(0);

  useEffect(() => {
    playPortalAmbient();

    return () => {
      stopPortalAmbient();
    };
  }, []);

  useEffect(() => {
    if (!loadingCitizenship && !loadingRelease) {
      if (currentRelease !== null) {
        if (isRelease) {
          setStateInfo(STATE_READY_TO_RELEASE);
        } else {
          setStateInfo(STATE_NEXT_UNFREEZE);
        }
      } else {
        setStateInfo(STATE_PROVE_ADDRESS);
      }
    }
  }, [loadingCitizenship, loadingRelease, isRelease, currentRelease]);

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

  const useSelectedGiftData = useMemo(() => {
    try {
      if (selectedAddress) {
        if (selectedAddress.match(PATTERN_CYBER) && totalGiftClaimed) {
          return { address: selectedAddress, ...totalGiftClaimed };
        }

        if (totalGift?.[selectedAddress]?.isClaimed) {
          return totalGift[selectedAddress];
        }
      }

      return null;
    } catch (error) {
      console.log('error', error);
      return null;
    }
  }, [selectedAddress, totalGift, totalGiftClaimed]);

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
    const content = Info({
      useReleasedStage,
      stepCurrent: stateInfo,
      nextRelease: useNextRelease,
    });

    if (error) {
      setAdviser(error, 'red');
    } else {
      setAdviser(content);
    }
  }, [setAdviser, useReleasedStage, stateInfo, useNextRelease, error]);

  const useUnClaimedGiftData = useMemo(() => {
    if (giftData !== null && citizenship && Object.keys(giftData.unClaimed.addresses).length > 0) {
      if (currentBonus?.current) {
        giftData.unClaimed.claim = Math.floor(giftData.unClaimed.amount * currentBonus.current);
        return { ...giftData.unClaimed, address: citizenship.owner };
      }
    }
    return null;
  }, [giftData, currentBonus, citizenship]);

  if (loadingCitizenship || loading) {
    return <div>...</div>;
  }

  let content;

  if (stateInfo !== STATE_INIT_NULL_ACTIVE) {
    content = (
      <>
        <PasportCitizenship
          txHash={txHash}
          citizenship={citizenship}
          updateFunc={setSelectedAddress}
          initStateCard={false}
          totalGift={totalGift}
        />

        {useUnClaimedGiftData !== null && (
          <CurrentGift
            title="Unclaimed"
            valueTextResult="unclaimed"
            initStateCard={false}
            selectedAddress={selectedAddress}
            currentGift={useUnClaimedGiftData}
          />
        )}

        <ReleaseStatus
          data={useReleasedStage}
          progress={progressClaim}
          amountGiftValue={useReleasedStage.gift}
          nextRelease={useNextRelease}
        />
      </>
    );
  }

  return (
    <>
      <MainContainer>
        <Stars />
        {!mobile && <MoonAnimation />}

        {content}
      </MainContainer>

      <ActionBarRelease
        txHash={txHash}
        addressActive={addressActive}
        updateTxHash={updateTxHash}
        isRelease={isRelease}
        callback={(err: string) => {
          setError(err);
        }}
        selectedAddress={selectedAddress}
        currentRelease={currentRelease}
        availableRelease={availableRelease}
        totalGift={totalGift}
        totalRelease={totalRelease}
        loadingRelease={loadingRelease}
      />
    </>
  );
}

export default Release;
