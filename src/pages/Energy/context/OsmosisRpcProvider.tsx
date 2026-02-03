import React, { useContext } from 'react';
import { osmosis } from 'osmojs';
import { useQuery } from '@tanstack/react-query';
import { Option } from 'src/types';
import { CHAIN_ID_OSMO, RPC_OSMO } from '../constants';

const { createRPCQueryClient } = osmosis.ClientFactory;

type Client = Awaited<ReturnType<typeof createRPCQueryClient>>;

const QueryClientContext = React.createContext<Option<Client>>(undefined);

export function useOsmosisRpc() {
  return useContext(QueryClientContext);
}

function OsmosisRpcProvider({ children }: { children: React.ReactNode }) {
  const { data, error, isFetching } = useQuery({
    queryKey: [CHAIN_ID_OSMO, 'connect'],
    queryFn: async () => {
      return createRPCQueryClient({
        rpcEndpoint: RPC_OSMO,
      });
    },
  });

  if (isFetching) {
    return null;
  }

  if (error) {
    console.error('Error queryClient connect: ', error.message);
  }

  return (
    <QueryClientContext.Provider value={data}>
      {children}
    </QueryClientContext.Provider>
  );
}

export default OsmosisRpcProvider;
