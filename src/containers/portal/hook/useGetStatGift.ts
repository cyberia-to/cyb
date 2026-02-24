import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { AMOUNT_ALL_STAGE, getConfigGift, getStateGift } from '../utils';

const initStateBonus = {
  up: 0,
  down: 0,
  current: 0,
};

const initClaimStatus = {
  targetClaim: 0,
  citizensClaim: 0,
};

function useGetStatGift() {
  const queryClient = useQueryClient();
  const [loading, setLoading] = useState(true);
  const [currentBonus, setCurrentBonus] = useState(initStateBonus);
  const [claimStat, setClaimStat] = useState(initClaimStatus);
  const [currentStage, setCurrentStage] = useState(0);
  const [progressClaim, setProgressClaim] = useState(0);

  useEffect(() => {
    const cheeckStateRelease = async () => {
      setLoading(true);
      if (queryClient) {
        try {
          const queryResponseResultConfig = await getConfigGift(queryClient);
          const queryResponseResultState = await getStateGift(queryClient);

          const validResponse =
            queryResponseResultState !== null && queryResponseResultConfig !== null;

          if (validResponse) {
            const {
              target_claim: targetClaim,
              coefficient_down: down,
              coefficient_up: up,
            } = queryResponseResultConfig;
            const { claims, coefficient } = queryResponseResultState;

            if (coefficient && down && up) {
              setCurrentBonus({
                current: parseFloat(coefficient),
                down: parseFloat(down),
                up: parseFloat(up),
              });
            }

            if (targetClaim && claims) {
              setClaimStat({ citizensClaim: claims, targetClaim });

              const currentStageTemp = new BigNumber(claims)
                .dividedBy(targetClaim)
                .multipliedBy(100)
                .dp(0, BigNumber.ROUND_FLOOR)
                .toNumber();

              setCurrentStage(
                currentStageTemp > AMOUNT_ALL_STAGE ? AMOUNT_ALL_STAGE : currentStageTemp
              );

              const curentProgressClaim = new BigNumber(claims)
                .dividedBy(targetClaim)
                .multipliedBy(100)
                .dp(0, BigNumber.ROUND_FLOOR)
                .toNumber();

              setProgressClaim(curentProgressClaim);
            }
            setLoading(false);
          }
        } catch (error) {
          console.log('error', error);
        }
      }
    };
    cheeckStateRelease();
  }, [queryClient]);

  return {
    loading,
    currentBonus,
    currentStage,
    claimStat,
    progressClaim,
  };
}

export default useGetStatGift;
