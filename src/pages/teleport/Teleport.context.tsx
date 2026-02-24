import React, { useMemo } from 'react';
import useCommunityPassports from 'src/features/passport/hooks/useCommunityPassports';
import useGetBalances from 'src/hooks/getBalances';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useGetTotalSupply from 'src/hooks/useGetTotalSupply';
import { Nullable } from 'src/types';
import { ObjKeyValue } from 'src/types/data';

const TeleportContext = React.createContext<{
  accountBalances: Nullable<ObjKeyValue>;
  totalSupplyProofList: Nullable<ObjKeyValue>;
  totalSupplyAll: Nullable<ObjKeyValue>;
  refreshBalances: () => void;
}>({
  accountBalances: null,
  totalSupplyProofList: undefined,
  totalSupplyAll: undefined,
  refreshBalances: () => undefined,
});

export const useTeleport = () => React.useContext(TeleportContext);

function TeleportContextProvider({ children }: { children: React.ReactNode }) {
  useCommunityPassports();
  const currentAddress = useCurrentAddress();

  const { totalSupplyProofList, totalSupplyAll } = useGetTotalSupply();
  const { liquidBalances: accountBalances, refresh: refreshBalances } =
    useGetBalances(currentAddress);

  const contextValue = useMemo(
    () => ({
      accountBalances,
      refreshBalances,
      totalSupplyProofList,
      totalSupplyAll,
    }),
    [accountBalances, refreshBalances, totalSupplyAll, totalSupplyProofList]
  );

  return <TeleportContext.Provider value={contextValue}>{children}</TeleportContext.Provider>;
}

export default TeleportContextProvider;
