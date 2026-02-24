import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { useSigningClient } from 'src/contexts/signerClient';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useAddToIPFS from 'src/features/ipfs/hooks/useAddToIPFS';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import { sendCyberlink } from 'src/services/neuron/neuronApi';

type Props = {
  toCid: string | undefined;
  fromCid: string | undefined;
};

function useCyberlink({ toCid, fromCid }: Props) {
  const { signer, signingClient } = useSigningClient();
  const { senseApi } = useBackend();

  const address = useCurrentAddress();

  const isReady = Boolean(signer && signingClient && address);

  const { data, isLoading, error, mutate } = useMutation<string>({
    mutationKey: ['cyberlink', toCid, fromCid, address],
    mutationFn: async () => {
      if (!isReady || !fromCid || !toCid) {
        return undefined;
      }

      const [{ address: signerAddress }] = await signer!.getAccounts();

      // if (e.message === 'Request rejected') {
      //     return;
      //   }
      // TODO: move somewhere
      if (signerAddress !== address) {
        throw new Error('Signer address is not equal to current account');
      }

      const transactionHash = await sendCyberlink(address, fromCid, toCid, {
        signingClient: signingClient!,
        senseApi,
      });

      return transactionHash;
    },
  });

  return {
    execute: mutate,
    isLoading,
    error,
    data,
    isReady,
  };
}

type Props2 = {
  to: any;
  from: any;
  callback: () => void;
};

// maybe move to new file
export function useCyberlinkWithWaitAndAdviser({ to, from, callback }: Props2) {
  const addToIPFSTo = useAddToIPFS(to);
  const addToIPFSFrom = useAddToIPFS(from);

  const fromCid = addToIPFSFrom.data;
  const toCid = addToIPFSTo.data;

  const cyberlink = useCyberlink({
    toCid,
    fromCid,
  });

  const { execute } = cyberlink;
  // think about this
  useEffect(() => {
    if (fromCid && toCid) {
      execute();
    }
  }, [fromCid, toCid, execute]);

  const waitForTx = useWaitForTransaction({
    hash: cyberlink.data,
    onSuccess: () => {
      callback({ toCid, fromCid });
    },
  });

  const isLoading =
    addToIPFSTo.isLoading || addToIPFSFrom.isLoading || cyberlink.isLoading || waitForTx.isLoading;
  const error = addToIPFSTo.error || addToIPFSFrom.error || cyberlink.error || waitForTx.error;

  const isReady = addToIPFSTo.isReady && addToIPFSFrom.isReady && cyberlink.isReady;

  useAdviserTexts({
    isLoading,
    error: error?.message || error,
    txHash: cyberlink.data,
    successText: waitForTx.data && 'Cyberlink successful',
  });

  return {
    isReady,
    execute: () => {
      Promise.all([addToIPFSTo.execute(), addToIPFSFrom.execute()]);
    },
    isLoading,
    // error if will need
  };
}

export default useCyberlink;
