import { CyberClient } from '@cybercongress/cyber-js';
import { QueryLiquidityPoolsResponse } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/query';
import { useQuery } from '@tanstack/react-query';
import { Option } from 'src/types';
import { Networks } from 'src/types/networks';

function useGetPoolsWarp(queryClient?: CyberClient) {
  const { data } = useQuery({
    queryKey: ['useFindPool', 'pools', Networks.BOSTROM],
    queryFn: () => queryClient?.pools() as Option<QueryLiquidityPoolsResponse>,
    enabled: Boolean(queryClient),
  });

  return { data };
}

export default useGetPoolsWarp;
