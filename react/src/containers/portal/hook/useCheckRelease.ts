/* eslint-disable no-await-in-loop */
/* eslint-disable no-restricted-syntax */
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { Nullable } from 'src/types';
import { getReleaseState } from '../utils';

export type TotalRelease = {
  stage: number;
  balanceClaim: number;
  addressOwner: string;
  address: string;
};

type ArrayTotalRelease = {
  [key: string]: TotalRelease;
};

function useCheckRelease(totalGift, addressActive, loadingGift, _updateFunc, currentStage) {
  const queryClient = useQueryClient();
  const [loadingRelease, setLoadingRelease] = useState(true);
  const [totalRelease, setTotalRelease] = useState<Nullable<ArrayTotalRelease>>(null);
  const [totalReadyRelease, setTotalReadyRelease] = useState<Nullable<TotalRelease[]>>(null);
  const [totalBalanceClaimAmount, setTotalBalanceClaimAmount] = useState(0);
  const [alreadyClaimed, setAlreadyClaimed] = useState(0);

  const initState = () => {
    setLoadingRelease(false);
    setTotalRelease(null);
    setTotalBalanceClaimAmount(0);
  };

  const calculationStateRelease = (dataQuery) => {
    const { stage, balance_claim: balanceClaim, address } = dataQuery;

    const releaseAddObj = {
      balanceClaim: 0,
      stage: 0,
      addressOwner: '',
    };

    if (balanceClaim) {
      releaseAddObj.balanceClaim = parseFloat(balanceClaim);
    }

    if (stage) {
      releaseAddObj.stage = parseFloat(stage);
    }

    if (address) {
      releaseAddObj.addressOwner = address;
    }

    return releaseAddObj;
  };

  useEffect(() => {
    const checkReleaseFunc = async () => {
      if (!loadingGift && totalGift !== undefined && totalGift === null) {
        initState();
      } else if (queryClient && addressActive && totalGift && Object.keys(totalGift).length > 0) {
        const { bech32 } = addressActive;
        setLoadingRelease(true);
        const result: ArrayTotalRelease = {};
        let balanceClaimAmount = 0;
        let alreadyClaimedAmount = 0;
        const totalReady: TotalRelease[] = [];
        for (const key in totalGift) {
          if (Object.hasOwn(totalGift, key)) {
            const element: { address: string; isClaimed: boolean } = totalGift[key];
            const { address, isClaimed } = element;
            if (isClaimed) {
              const queryResponseResultRelease = await getReleaseState(queryClient, address);

              if (queryResponseResultRelease) {
                const calculationState = calculationStateRelease(queryResponseResultRelease);
                const { balanceClaim, stage, addressOwner } = calculationState;
                if (balanceClaim && bech32 === addressOwner) {
                  balanceClaimAmount += balanceClaim;
                }

                if (bech32 !== addressOwner && balanceClaim) {
                  alreadyClaimedAmount += balanceClaim;
                }
                if (stage < currentStage) {
                  totalReady.push({ address, ...calculationState });
                }
                result[address] = { address, ...calculationState };
              }
            }
          }
        }
        setAlreadyClaimed(alreadyClaimedAmount);
        setTotalBalanceClaimAmount(balanceClaimAmount);
        if (Object.keys(result).length > 0) {
          setTotalRelease(result);
        } else {
          setTotalRelease(null);
        }

        if (totalReady.length > 0) {
          setTotalReadyRelease(totalReady);
        } else {
          setTotalReadyRelease(null);
        }
        setLoadingRelease(false);
      }
    };
    checkReleaseFunc();
  }, [
    queryClient,
    totalGift,
    loadingGift,
    addressActive,
    calculationStateRelease,
    currentStage,
    initState,
  ]);

  return {
    totalRelease,
    totalBalanceClaimAmount,
    alreadyClaimed,
    totalReadyRelease,
    loadingRelease,
  };
}

export default useCheckRelease;
