import { UseQueryOptions, useQuery } from '@tanstack/react-query';
import { useQueryClient } from 'src/contexts/queryClient';
import { CybernetContractQuery, queryCybernetContract } from '../api';
import { useCybernet } from './cybernet.context';

type Props = {
  query: CybernetContractQuery;
  contractAddress?: string;

  // combine to 1 object prop
  skip?: boolean;
  refetchInterval?: UseQueryOptions['refetchInterval'];
};

// TODO: copied from usePassportContract, reuse  core logic

function useQueryCybernetContract<DataType>({
  query,
  contractAddress,
  skip,
  refetchInterval,
}: Props) {
  const queryClient = useQueryClient();

  const { selectedContract } = useCybernet();
  const contractAddr = contractAddress || selectedContract?.address;

  const { refetch, data, error, isLoading } = useQuery<unknown, unknown, DataType>(
    ['queryCybernetContract', contractAddr, query],
    () => {
      return queryCybernetContract(contractAddr, query, queryClient!);
    },
    {
      // @ts-expect-error
      refetchInterval,
      enabled: !skip && Boolean(queryClient && contractAddr),
    }
  );

  return {
    data,
    loading: isLoading,
    error,
    refetch,
  };
}

export default useQueryCybernetContract;
