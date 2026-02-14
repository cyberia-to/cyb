import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { useCyberClient } from 'src/contexts/queryCyberClient';
import { fromAscii } from '@cosmjs/encoding';
import { Decimal } from '@cosmjs/math';
import BigNumber from 'bignumber.js';

function useValidatorStakingProvisions() {
  const { rpc } = useCyberClient();
  const { data: annualProvisions, isFetching } = useQuery({
    queryKey: ['mint', 'annualProvisions'],
    queryFn: () => rpc.cosmos.mint.v1beta1.annualProvisions(),
    enabled: Boolean(rpc),
    select: (data) =>
      Decimal.fromAtomics(fromAscii(data.annualProvisions), 18).toString(),
  });
  const { data: communityTax } = useQuery({
    queryKey: ['distribution', 'params'],
    queryFn: () => rpc.cosmos.distribution.v1beta1.params(),
    enabled: Boolean(rpc),
    select: (data) => data.params.communityTax,
  });

  const stakingProvisions = useMemo(() => {
    if (!annualProvisions || !communityTax) {
      return undefined;
    }

    return new BigNumber(annualProvisions)
      .multipliedBy(new BigNumber(1).minus(communityTax))
      .toString(10);
  }, [annualProvisions, communityTax]);

  return { stakingProvisions, isFetching };
}

export default useValidatorStakingProvisions;
