import { useQuery } from '@tanstack/react-query';
import { useQueryClient } from 'src/contexts/queryClient';

async function fetchGovParam(queryClient, query) {
  const response = await queryClient.govParams(query);
  return response;
}

export function useGovParam(query) {
  const queryClient = useQueryClient();

  const { data, error, isLoading } = useQuery(
    ['govParam', query],
    () => fetchGovParam(queryClient, query),
    {}
  );

  let paramData;
  if (query === 'deposit') {
    paramData = data ? parseFloat(data.depositParams.minDeposit[0].amount) : 0;
  } else {
    paramData = data;
  }

  return {
    paramData,
    isLoading,
    error,
  };
}
