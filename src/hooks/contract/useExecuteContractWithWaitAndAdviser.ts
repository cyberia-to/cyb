import { Coin } from '@cosmjs/launchpad';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import useAdviserTexts from '../../features/adviser/useAdviserTexts';
import useExecuteContract from './useExecuteContract';

export type Props = {
  contractAddress: string;
  query: any;
  funds?: Coin[] | undefined;
  onSuccess?: (response: any) => void;
};

function useExecuteContractWithWaitAndAdviser({
  contractAddress,
  query,
  funds,
  onSuccess,
  successMessage,
}: Props) {
  const { isLoading, isReady, error, mutate, transactionHash } = useExecuteContract({
    contractAddress,
    query,
    funds,
  });

  const waitForTx = useWaitForTransaction({
    hash: transactionHash,
    onSuccess,
  });

  const e = error || waitForTx.error;
  useAdviserTexts({
    isLoading: isLoading || waitForTx.isLoading,
    loadingText: isLoading
      ? 'Wallet confirmation'
      : waitForTx.isLoading
        ? 'Transaction confirmation'
        : undefined,
    error: e,
    successText: !!waitForTx.data && successMessage,
    txHash: e && transactionHash,
    priority: true,
  });

  return {
    mutate,
    isReady,
    isLoading: isLoading || waitForTx.isLoading,
  };
}

export default useExecuteContractWithWaitAndAdviser;
