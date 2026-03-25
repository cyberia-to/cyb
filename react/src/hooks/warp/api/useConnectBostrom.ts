import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import { Networks } from 'src/types/networks';

function useConnectBostrom() {
  const { data: queryClient } = useQuery({
    queryKey: ['cyberClient', 'connect', Networks.BOSTROM],
    queryFn: () => CyberClient.connect(RPC_URL),
    enabled: CHAIN_ID !== Networks.BOSTROM,
  });

  return { queryClient };
}

export default useConnectBostrom;
