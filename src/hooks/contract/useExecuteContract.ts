import { Coin } from '@cosmjs/launchpad';
import { useMutation } from '@tanstack/react-query';
import { keplrConstants } from 'src/constants/keplr';
import { useSigningClient } from 'src/contexts/signerClient';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';

type Props = {
  contractAddress: string;
  query: any;
  funds?: Coin[] | undefined;
};

function useExecuteContract({ contractAddress, query, funds }: Props) {
  const { signer, signingClient } = useSigningClient();

  const { mutate, data, isLoading, error } = useMutation({
    // mutationKey: 'useExecuteContract',
    mutationFn: async () => {
      if (!signer || !signingClient) {
        return Promise.reject('No signer or signing client');
      }

      const [{ address }] = await signer.getAccounts();

      return signingClient.execute(
        address,
        contractAddress,
        query,
        Soft3MessageFactory.fee(2),
        '',
        funds
      );
    },
  });

  let formattedError = error?.message;
  if (formattedError === keplrConstants.errors.REQUEST_REJECTED) {
    formattedError = '';
  }

  return {
    isLoading,
    isReady: Boolean(signer && signingClient),
    transactionHash: data?.transactionHash as string | undefined,
    error: formattedError,
    mutate,
  };
}

export default useExecuteContract;
