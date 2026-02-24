import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

function useQueryClientMethod<T extends keyof CyberClient>(
  methodName: T,
  params?: Parameters<CyberClient[T]>
) {
  const queryClient = useQueryClient();

  // TODO: think how to memo params correctly
  const memoParams = useMemo(() => params, [params]);

  const { isLoading, data, error } = useQuery<
    unknown,
    unknown,
    Awaited<ReturnType<CyberClient[T]>>
  >(
    ['queryClientMethod', methodName, memoParams],
    () => {
      const func = queryClient![methodName].bind(queryClient);

      // refactor
      if (memoParams) {
        return func(...memoParams);
      }

      return func();
    },
    {
      enabled: !!queryClient,
    }
  );

  return {
    error,
    data,
    loading: isLoading,
  };
}

export default useQueryClientMethod;
