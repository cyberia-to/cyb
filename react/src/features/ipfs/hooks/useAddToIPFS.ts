import { useMutation } from '@tanstack/react-query';
import { useBackend } from 'src/contexts/backend/backend';
import { addIfpsMessageOrCid } from 'src/utils/ipfs/helpers';

function useAddToIPFS(content: any) {
  const { isIpfsInitialized, ipfsApi } = useBackend();

  const isReady = Boolean(isIpfsInitialized && ipfsApi);

  const { data, isLoading, error, mutateAsync } = useMutation<string>({
    mutationKey: ['addToIPFS', content],
    mutationFn: async () => {
      if (!isReady) {
        return;
      }

      return addIfpsMessageOrCid(content, { ipfsApi });
    },
  });

  return {
    execute: mutateAsync,
    isLoading,
    error,
    data,
    isReady,
  };
}

export default useAddToIPFS;
