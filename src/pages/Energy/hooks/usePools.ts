import { useQuery } from '@tanstack/react-query';
import { Pool } from 'osmojs/osmosis/gamm/v1beta1/balancerPool';
import { useOsmosisRpc } from '../context/OsmosisRpcProvider';
import { paginate } from '../utils/utils';
import { Osmosis } from '../utils/assets';

export type PoolList = Pool[] & {
  priced: Pool[];
};

function usePools() {
  const rpc = useOsmosisRpc();
  const { data, refetch } = useQuery(
    ['osmosis', 'pools'],
    async () => {
      return rpc?.osmosis.poolmanager.v1beta1.allPools({
        pagination: paginate(5000n),
      });
    },
    { enabled: Boolean(rpc), refetchInterval: 1_000 * 60 * 3 }
  );

  // rpc?.osmosis.poolmanager.v1beta1.estimateSinglePoolSwapExactAmountIn();

  const all: Pool[] = data?.pools || [];
  const map = all.reduce(
    (map, pool) => map.set(pool.id, pool),
    new Map<bigint, Pool>()
  );
  const freefloat = (all.filter(({ $typeUrl }) =>
    $typeUrl?.includes('/osmosis.gamm.v1beta1.Pool')
  ) || []) as PoolList;

  freefloat.priced = freefloat.filter(({ poolAssets }) =>
    poolAssets.every(({ token }) => Osmosis.CoinDenomToAsset[token.denom])
  );

  return { all, map, freefloat: freefloat.priced, refetch };
}

export default usePools;
