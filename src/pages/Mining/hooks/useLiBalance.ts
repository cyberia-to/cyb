import { useQuery } from '@tanstack/react-query';
import { useQueryClient } from 'src/contexts/queryClient';
import { LI_DENOM } from 'src/constants/mining';

function useLiBalance(address: string | undefined) {
  const queryClient = useQueryClient();

  const { data, isLoading, refetch } = useQuery(
    ['liBalance', address],
    async () => {
      if (!queryClient || !address) return 0;
      const balances = await queryClient.getAllBalances(address);
      const li = balances.find((b) => b.denom === LI_DENOM);
      return li ? Number(li.amount) / 1_000_000 : 0;
    },
    {
      enabled: !!queryClient && !!address,
    }
  );

  return {
    balance: data ?? 0,
    loading: isLoading,
    refetch,
  };
}

export default useLiBalance;
