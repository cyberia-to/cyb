import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { useCyberClient } from 'src/contexts/queryCyberClient';

export type Props = {
  hash: string | null | undefined;
  onSuccess?: (response: any) => void;
};

function useWaitForTransaction({ hash, onSuccess }: Props) {
  const { rpc } = useCyberClient();
  const onSuccessRef = useRef(onSuccess);
  onSuccessRef.current = onSuccess;

  const { data, isFetching, error } = useQuery({
    queryKey: ['waitForTx', hash],
    queryFn: () => rpc.cosmos.tx.v1beta1.getTx({ hash: hash! }),
    enabled: Boolean(hash && rpc),
    retry: (_: number, error: Error) => error.message.includes('NotFound'),
    retryDelay: 2500,
  });

  useEffect(() => {
    if (data && onSuccessRef.current) {
      onSuccessRef.current(data);
    }
  }, [data]);

  return {
    data,
    error: error ? (error as Error).message : undefined,
    isLoading: isFetching,
  };
}

export default useWaitForTransaction;
