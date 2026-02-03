import React, { useCallback, useContext, useMemo } from 'react';
import { CoinDenom, PriceHash } from '@osmonauts/math/types';
import { Coin } from '@cosmjs/stargate';
import useBalances from '../hooks/useBalances';
import useSwap from '../hooks/useSwap';
import { EnergyPackageSwapRoutes } from '../types/EnergyPackages';
import { useOsmosisSign } from './OsmosisSignerProvider';

const EnergyContext = React.createContext<{
  prices: PriceHash;
  balances: {
    assets: Coin[];
    hash: Record<CoinDenom, Coin>;
  };
  energyPackageSwapRoutes?: EnergyPackageSwapRoutes[];
  refetch: () => void;
}>({
  prices: {},
  balances: { assets: [], hash: {} },
  energyPackageSwapRoutes: [],
  refetch: () => {},
});

export function useEnergy() {
  return useContext(EnergyContext);
}

function EnergyProvider({ children }: { children: React.ReactNode }) {
  const { address } = useOsmosisSign();
  const { assets, hash, refetch: refetchBal } = useBalances(address);
  const { energyPackageSwapRoutes, refetchSwapRoute, prices } = useSwap();

  const refetchFnc = useCallback(() => {
    refetchBal();
    refetchSwapRoute();
  }, [refetchBal, refetchSwapRoute]);

  const value = useMemo(
    () => ({
      prices,
      balances: { assets, hash },
      energyPackageSwapRoutes,
      refetch: refetchFnc,
    }),
    [prices, assets, hash, energyPackageSwapRoutes, refetchFnc]
  );

  return (
    <EnergyContext.Provider value={value}>{children}</EnergyContext.Provider>
  );
}

export default EnergyProvider;
