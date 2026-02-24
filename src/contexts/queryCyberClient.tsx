import { createRpcQueryHooks, cyber, useRpcClient } from '@cybercongress/cyber-ts';
import { useQuery } from '@tanstack/react-query';
import React, { useContext } from 'react';
import { RPC_URL } from 'src/constants/config';

const QueryClientContext = React.createContext<{
  rpc: Awaited<ReturnType<typeof cyber.ClientFactory.createRPCQueryClient>>;
  hooks: ReturnType<typeof createRpcQueryHooks>;
}>(undefined);

// example
// const { hooks, rpc } = useCyberTsQueryClient();

// rpc.cosmos.tx.v1beta1.getTxsEvent({
//   // orderBy:
// })

// const query = hooks.cyber.rank.v1beta1.useTop({
//   request: {
//     pagination: {
//       page: 0,
//       perPage: 50,
//     },
//   },
// });

// eslint-disable-next-line import/no-unused-modules
export function useCyberClient() {
  return useContext(QueryClientContext);
}

const rpcEndpoint = RPC_URL;

function CyberTsQueryClientProvider({ children }: { children: React.ReactNode }) {
  const { data, error, isFetching } = useQuery({
    queryKey: ['cyberTsClient', 'connect'],
    queryFn: () => {
      const { createRPCQueryClient } = cyber.ClientFactory;

      return createRPCQueryClient({
        rpcEndpoint,
      });
    },
  });

  if (error) {
    console.error('Error queryClient connect: ', error.message);
  }

  const { data: rpcClient } = useRpcClient({
    rpcEndpoint,
    options: {
      enabled: !!rpcEndpoint,
    },
  });

  if (!rpcClient) {
    return null;
  }

  const hooks = createRpcQueryHooks({
    rpc: rpcClient,
  });

  return (
    <QueryClientContext.Provider
      value={{
        rpc: data,
        hooks,
      }}
    >
      {children}
    </QueryClientContext.Provider>
  );
}

export default CyberTsQueryClientProvider;
