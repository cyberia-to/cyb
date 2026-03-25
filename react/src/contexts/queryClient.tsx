import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import React, { useContext } from 'react';
import { RPC_URL } from 'src/constants/config';
import { Option } from 'src/types';

const QueryClientContext = React.createContext<Option<CyberClient>>(undefined);

/**
 * @deprecated use queryCyberClient
 */
export function useQueryClient() {
  return useContext(QueryClientContext);
}

function QueryClientProvider({ children }: { children: React.ReactNode }) {
  const {
    data: client,
    error,
    isFetching,
  } = useQuery({
    queryKey: ['cyberClient', 'connect'],
    queryFn: async () => {
      console.log('[QueryClient] Connecting to RPC:', RPC_URL);
      const c = await CyberClient.connect(RPC_URL);
      console.log('[QueryClient] Connected successfully');
      return c;
    },
    retry: 3,
    retryDelay: (attempt) => Math.min(2000 * 2 ** attempt, 15000),
  });

  if (error) {
    console.error('Error queryClient connect: ', error.message);
  }

  return <QueryClientContext.Provider value={client}>{children}</QueryClientContext.Provider>;
}

export default QueryClientProvider;
