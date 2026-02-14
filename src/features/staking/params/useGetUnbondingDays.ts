import BigNumber from 'bignumber.js';
import { useCyberClient } from 'src/contexts/queryCyberClient';

function useGetUnbondingDays() {
  const { hooks } = useCyberClient();

  const { data: stakingParamsData } = hooks.cosmos.staking.v1beta1.useParams({
    request: {},
  });

  const unbondingDays =
    stakingParamsData &&
    new BigNumber(
      BigInt(stakingParamsData.params.unbondingTime.seconds).toString()
    )
      .dividedBy(60)
      .dividedBy(60)
      .dividedBy(24)
      .toNumber();

  return { unbondingDays };
}

export default useGetUnbondingDays;
