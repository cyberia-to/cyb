import { useQuery } from '@tanstack/react-query';
import { useQueryClient } from 'src/contexts/queryClient';

export const useGetKarma = (address: string) => {
  const queryClient = useQueryClient();

  const { data } = useQuery({
    queryKey: ['karma', address],
    queryFn: async () => {
      try {
        const response = await queryClient!.karma(address);

        return response.karma;
      } catch {
        return null;
      }
    },
    enabled: Boolean(queryClient && address),
  });

  return { data };
};
