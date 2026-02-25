import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import { useRef } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

function useQueryClientMethod<T extends keyof CyberClient>(
  methodName: T,
  params?: Parameters<CyberClient[T]>
) {
  const queryClient = useQueryClient();

  // Stabilize params by comparing JSON content instead of reference
  const paramsJson = JSON.stringify(params);
  const stableParamsRef = useRef(params);
  const prevJsonRef = useRef(paramsJson);
  if (paramsJson !== prevJsonRef.current) {
    prevJsonRef.current = paramsJson;
    stableParamsRef.current = params;
  }
  const stableParams = stableParamsRef.current;

  const { isLoading, data, error, refetch, dataUpdatedAt } = useQuery<
    unknown,
    unknown,
    Awaited<ReturnType<CyberClient[T]>>
  >(
    ['queryClientMethod', methodName, paramsJson],
    () => {
      const func = queryClient![methodName].bind(queryClient);

      if (stableParams) {
        return func(...stableParams);
      }

      return func();
    },
    {
      enabled: !!queryClient,
      keepPreviousData: true,
      refetchInterval: 15_000,
      staleTime: 10_000,
    }
  );

  return {
    error,
    data,
    loading: isLoading,
    refetch,
    dataUpdatedAt,
  };
}

export default useQueryClientMethod;
