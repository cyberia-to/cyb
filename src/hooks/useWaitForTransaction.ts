import { IndexedTx } from '@cosmjs/stargate';
import { useCyberClient } from 'src/contexts/queryCyberClient';

// from
// https://wagmi.sh/react/hooks/useWaitForTransaction

export type Props = {
  hash: string | null | undefined;
  onSuccess?: (response: IndexedTx) => void;
};

// const NO_RESPONSE_ERROR = 'No response';

function useWaitForTransaction({ hash, onSuccess }: Props) {
  const { hooks } = useCyberClient();

  const { data, isFetching, error } = hooks.cosmos.tx.v1beta1.useGetTx({
    request: {
      hash: hash!,
    },
    options: {
      enabled: Boolean(hash),
      retry: (_, error) => {
        return error.message.includes('NotFound');
      },
      retryDelay: 2500,
      onSuccess: (response) => {
        onSuccess?.(response);
      },
    },
  });

  // const { data, isFetching, error } = useQuery(
  //   ['tx', hash],
  //   async () => {
  //     const response = await queryClient!.getTx(hash!);

  //     if (!response) {
  //       // seems not working with retry and retryDelay
  //       throw new Error(NO_RESPONSE_ERROR);
  //     } else if (response.code !== 0) {
  //       throw new Error(response.rawLog);
  //     }

  //     return response;
  //   },
  //   {
  //     enabled: Boolean(queryClient && hash),
  //     retry: (_, error) => {
  //       return error.message === NO_RESPONSE_ERROR;
  //     },
  //     retryDelay: 2500,
  //     onSuccess: (response) => {
  //       onSuccess && onSuccess(response);
  //     },
  //   }
  // );

  // const formattedData = useMemo(() => {
  //   if (data) {

  //     return
  //   }
  // }, [data]);

  return {
    data,
    error: error?.message,
    isLoading: isFetching,
  };
}

export default useWaitForTransaction;
