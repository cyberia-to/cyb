import { useSigningClient } from 'src/contexts/signerClient';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';

/**
 * Hook that provides the connected wallet's signer for mining.
 * Uses the same account the user connected via Keplr (or mnemonic fallback).
 */
export default function useAutoSigner() {
  const { signer, signingClient } = useSigningClient();
  const address = useAppSelector(selectCurrentAddress);

  return { signer: signer ?? null, signingClient: signingClient ?? null, address };
}
