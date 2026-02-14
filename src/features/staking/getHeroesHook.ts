import { Validator } from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';
import { useQueries } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useCallback, useMemo } from 'react';
import { useCyberClient } from 'src/contexts/queryCyberClient';
import BondStatusKey from '../../pages/Sphere/types/bondStatus';

const sortFunc = (itemA: Validator, itemB: Validator) => {
  return new BigNumber(itemB.tokens)
    .minus(new BigNumber(itemA.tokens))
    .toNumber();
};

function useGetHeroes() {
  const { rpc } = useCyberClient();

  const queries = useQueries({
    queries: Object.keys(BondStatusKey).map((status) => {
      return {
        queryKey: ['validators', status],
        queryFn: () =>
          rpc.cosmos.staking.v1beta1.validators({ status }).then((response) => {
            return response.validators.sort(sortFunc);
          }),
        enabled: Boolean(rpc),
      };
    }),
  });

  const loading = useMemo(
    () => queries.some((query) => query.isLoading),
    [queries]
  );
  const error = useMemo(
    () => queries.find((query) => !!query.error)?.error,
    [queries]
  );

  const data = useMemo(
    () =>
      queries
        .map((query) => query.data)
        .filter((data) => typeof data !== 'undefined')
        .flat(),
    [queries]
  );

  const active = useMemo(() => data.filter((item) => !item.jailed), [data]);
  const jailed = useMemo(() => data.filter((item) => item.jailed), [data]);

  const refetchAll = useCallback(() => {
    queries.forEach((result) => result.refetch());
  }, [queries]);

  return {
    validators: data,
    countHeroes: { active: active.length, jailed: jailed.length },
    loadingValidators: loading,
    error,
    refetchAll,
  };
}

export default useGetHeroes;
