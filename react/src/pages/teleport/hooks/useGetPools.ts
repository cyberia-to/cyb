import { Params } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { QueryParamsResponse } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/query';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { Option } from 'src/types';

function useGetParams() {
  const queryClient = useQueryClient();
  const { data } = useQuery(
    ['liquidityParams'],
    () => {
      return queryClient?.liquidityParams() as Option<QueryParamsResponse>;
    },
    {
      enabled: Boolean(queryClient),
    }
  );
  const [params, setParams] = useState<Params>();

  useEffect(() => {
    const getLiquidityParams = () => {
      if (data !== undefined) {
        setParams(data.params);
      }
    };
    getLiquidityParams();
  }, [data]);

  return params;
}

export default useGetParams;
