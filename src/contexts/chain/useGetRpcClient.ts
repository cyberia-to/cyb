import {
  cyber,
  createRpcQueryHooks,
  useRpcClient,
} from '@cybercongress/cyber-ts';

import { useQuery } from '@tanstack/react-query';
import defaultNetworks from 'src/constants/defaultNetworks';
import { Networks } from 'src/types/networks';

type GetRpcClientType = {
  rpc: Awaited<ReturnType<typeof cyber.ClientFactory.createRPCQueryClient>>;
  hooks: ReturnType<typeof createRpcQueryHooks>;
};

function useGetRpcClient(
  chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
): GetRpcClientType | undefined {
  const rpcEndpoint = defaultNetworks[chainId].RPC_URL;

  const { data, error, isFetching } = useQuery({
    queryKey: ['cyberTsClient', 'connect', chainId],
    queryFn: () => {
      const { createRPCQueryClient } = cyber.ClientFactory;

      return createRPCQueryClient({
        rpcEndpoint,
      });
    },
    enabled: Boolean(rpcEndpoint),
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
    return undefined;
  }

  const hooks = createRpcQueryHooks({
    rpc: rpcClient,
  });

  return {
    hooks,
    rpc: data,
  };
}

export default useGetRpcClient;
