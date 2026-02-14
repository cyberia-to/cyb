import { useQuery } from '@tanstack/react-query';
import { useCyberClient } from 'src/contexts/queryCyberClient';

function useStakingPool() {
  const { rpc } = useCyberClient();

  const { data } = useQuery({
    queryKey: ['staking', 'pool'],
    queryFn: () => rpc.cosmos.staking.v1beta1.pool(),
    enabled: Boolean(rpc),
  });

  const bondedTokens = data ? parseFloat(data.pool.bondedTokens) : 0;

  return { bondedTokens };
}

export default useStakingPool;
