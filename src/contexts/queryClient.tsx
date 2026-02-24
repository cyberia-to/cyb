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
      return CyberClient.connect(RPC_URL);
    },
  });

  if (isFetching) {
    return null;
  }

  if (error) {
    console.error('Error queryClient connect: ', error.message);

    return <span>api connection error</span>;
    // return <APIError />;
  }

  return <QueryClientContext.Provider value={client}>{children}</QueryClientContext.Provider>;
}

export default QueryClientProvider;
