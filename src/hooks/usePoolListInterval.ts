import { QueryLiquidityPoolsResponse } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/query';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { setPools } from 'src/redux/features/warp';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';

type OptionPoolListInterval = {
  refetchInterval?: number | false;
};

const keyQuery = 'liquidityPools';

function usePoolListInterval(option: OptionPoolListInterval = { refetchInterval: false }) {
  const queryClient = useQueryClient();
  const dispatch = useAppDispatch();
  const { pools: poolsLS } = useAppSelector((state) => state.warp);

  const [dataPool, setDataPool] = useState<Option<QueryLiquidityPoolsResponse>>(poolsLS);

  const { data } = useQuery(
    [keyQuery],
    () => {
      return queryClient?.pools() as Option<QueryLiquidityPoolsResponse>;
    },
    {
      enabled: Boolean(queryClient),
      refetchInterval: option.refetchInterval,
    }
  );

  useEffect(() => {
    if (data) {
      setDataPool(data);
      dispatch(setPools(data));
    }
  }, [data, dispatch]);

  return dataPool?.pools;
}

export default usePoolListInterval;
