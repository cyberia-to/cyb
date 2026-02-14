import React, { useCallback, useMemo } from 'react';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { useGetBalance } from 'src/containers/sigma/hooks/utils';
import Loader2 from 'src/components/ui/Loader2';
import useValidatorStakingProvisions from '../../features/staking/params/useValidatorStakingProvisions';
import useStakingPool from '../../features/staking/useStakingPool';
import useDelegatorDelegations from '../../features/staking/delegation/useDelegatorDelegations';
import useGetUnbondingDays from '../../features/staking/params/useGetUnbondingDays';
import useGetHeroes from '../../features/staking/getHeroesHook';

// type ResponesGetHeroes = Omit<
//   ReturnType<typeof useGetHeroes>,
//   'countHeroes' | 'error'
// >;

const SphereContext = React.createContext<{
  bondedTokens: number;
  stakingProvisions?: string;
  balance?: ReturnType<typeof useGetBalance>['data'];
  isFetchingBalance: boolean;
  delegationsData: ReturnType<
    typeof useDelegatorDelegations
  >['delegationsData'];
  unbondingDays?: number;
  validators: ReturnType<typeof useGetHeroes>['validators'];
  refetchFunc: () => void;
}>({
  bondedTokens: 0,
  stakingProvisions: undefined,
  balance: undefined,
  isFetchingBalance: true,
  delegationsData: {},
  unbondingDays: undefined,
  validators: [],
  refetchFunc: () => {},
});

export const useSphereContext = () => React.useContext(SphereContext);

function SphereContextProvider({ children }: { children: React.ReactNode }) {
  // const { chainId = CHAIN_ID } = useParams();
  // const { signingClient, rpcClient } = useChain(chainId);

  const {
    validators,
    loadingValidators,
    refetchAll: refetchAllHeroes,
  } = useGetHeroes();

  const addressActive = useAppSelector(selectCurrentAddress);

  const {
    data: balance,
    refetch: refetchBalance,
    isFetching: isFetchingBalance,
  } = useGetBalance(addressActive);
  const { delegationsData, refetchDelegations } =
    useDelegatorDelegations(addressActive);
  const { bondedTokens } = useStakingPool();
  const { unbondingDays } = useGetUnbondingDays();

  const { stakingProvisions } = useValidatorStakingProvisions();

  const refetchFunc = useCallback(() => {
    refetchAllHeroes();
    refetchBalance();
    refetchDelegations();
  }, [refetchBalance, refetchDelegations, refetchAllHeroes]);

  const contextValue = useMemo(
    () => ({
      stakingProvisions,
      balance,
      isFetchingBalance,
      bondedTokens,
      delegationsData,
      unbondingDays,
      validators,
      refetchFunc,
    }),
    [
      stakingProvisions,
      balance,
      isFetchingBalance,
      bondedTokens,
      delegationsData,
      unbondingDays,
      validators,
      refetchFunc,
    ]
  );

  if (loadingValidators) {
    return <Loader2 />;
  }

  return (
    <SphereContext.Provider value={contextValue}>
      {children}
    </SphereContext.Provider>
  );
}

export default SphereContextProvider;
