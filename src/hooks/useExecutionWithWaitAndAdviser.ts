import { useMutation } from '@tanstack/react-query';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useWaitForTransaction from './useWaitForTransaction';

function useExecutionWithWaitAndAdviser(executeFn: () => Promise<any>, callback?: () => void) {
  const mutation = useMutation({
    // mutationKey: 'executeFn',
    mutationFn: executeFn,
  });

  const waitForTransaction = useWaitForTransaction({
    hash: mutation.data?.hash,
    onSuccess: callback,
  });

  let loadingText;

  if (mutation.isLoading) {
    loadingText = 'preparing transaction...';
  } else if (waitForTransaction.isLoading) {
    loadingText = 'confirming transaction...';
  }

  useAdviserTexts({
    isLoading: waitForTransaction.isLoading,
    error: waitForTransaction.error || mutation.error,
    loadingText,
  });
  return {
    execute: mutation.mutate,
    isLoading: mutation.isLoading || waitForTransaction.isLoading,
    // maybe isReady will need
    // error if will need
  };
}

export default useExecutionWithWaitAndAdviser;
