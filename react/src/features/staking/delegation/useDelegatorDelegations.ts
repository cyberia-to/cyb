import { Coin } from '@cosmjs/stargate';
import { useQuery } from '@tanstack/react-query';
import { useCyberClient } from 'src/contexts/queryCyberClient';
import { getDelegatorDelegations } from 'src/features/staking/delegation/getDelegatorDelegations';

function useDelegatorDelegations(addressActive?: string) {
  const { rpc } = useCyberClient();

  const { data: delegationsResponse, refetch: refetchDelegations } = useQuery({
    queryKey: ['staking', 'delegatorDelegations', addressActive],
    queryFn: () => getDelegatorDelegations(rpc, addressActive || ''),
    enabled: Boolean(rpc && addressActive),
  });

  const delegationsData = delegationsResponse
    ? delegationsResponse.reduce<{ [key: string]: Coin }>(
        (acc, item) => ({
          ...acc,
          [item.delegation.validatorAddress]: item.balance,
        }),
        {}
      )
    : {};

  return { delegationsData, refetchDelegations };
}

export default useDelegatorDelegations;
