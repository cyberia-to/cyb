import { SigningCyberClient } from '@cybercongress/cyber-js';
import React, { useContext, useEffect, useMemo, useState } from 'react';
import { Option } from 'src/types';
import { CHAIN_ID } from 'src/constants/config';
import { Networks } from 'src/types/networks';
import { useQuery } from '@tanstack/react-query';
import Loader2 from 'src/components/ui/Loader2';
import useGetRpcClient from './useGetRpcClient';
import { useSigningClient } from '../signerClient';

type ChainContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly rpcClient: ReturnType<typeof useGetRpcClient>;
  setChainId: (chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) => void;
};

const valueContext = {
  signingClient: undefined,
  rpcClient: undefined,
  setChainId: () => {},
};

const ChainContext = React.createContext<ChainContextType>(valueContext);

export function useChain(chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) {
  const { signingClient, rpcClient, setChainId } = useContext(ChainContext);

  useEffect(() => {
    setChainId(chainId);
  }, [chainId, setChainId]);

  return { signingClient, rpcClient };
}

export function useChainContext() {
  return useContext(ChainContext);
}

function ChainProvider({ children }: { children: React.ReactNode }) {
  const [chainId, setChainId] = useState<
    Networks.BOSTROM | Networks.SPACE_PUSSY
  >(CHAIN_ID);
  const { getSignClientByChainId } = useSigningClient();
  const dataRpc = useGetRpcClient(chainId);

  const { data: dataSigner } = useQuery(
    ['getCyberClientById', chainId],
    () => getSignClientByChainId(chainId),
    { enabled: Boolean(chainId) }
  );

  const value = useMemo(
    () => ({ signingClient: dataSigner, rpcClient: dataRpc, setChainId }),
    [dataSigner, dataRpc]
  );

  if (!dataSigner || !dataRpc) {
    return <Loader2 />;
  }

  return (
    <ChainContext.Provider value={value}>{children}</ChainContext.Provider>
  );
}

export default ChainProvider;
