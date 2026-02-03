import { useQuery } from '@tanstack/react-query';
import { Coin } from '@cosmjs/stargate';
import { useOsmosisRpc } from '../context/OsmosisRpcProvider';

function useBalances(address?: string) {
  const rpc = useOsmosisRpc();

  const { data, refetch } = useQuery(
    ['osmosis', 'allBalances', address],
    async () => {
      if (!address) {
        return undefined;
      }
      return rpc?.cosmos.bank.v1beta1.allBalances({
        address,
      });
    },
    { enabled: Boolean(rpc && address) }
  );

  const all = data?.balances || [];
  const hash = (all.reduce(
    (acc, coin) => ({ ...acc, [coin.denom]: coin }),
    {}
  ) || {}) as Record<Coin['denom'], Coin>;
  const pools = all.filter((coin) => coin.denom.startsWith('gamm')) || [];
  const assets = all.filter((coin) => !coin.denom.startsWith('gamm')) || [];

  return { all, hash, pools, assets, refetch };
}

export default useBalances;
