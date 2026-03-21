import { SigningCyberClient } from '@cybercongress/cyber-js';
import { OfflineSigner } from '@cybercongress/cyber-js/build/signingcyberclient';
import React, { useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import defaultNetworks, { getHealthyRpcUrl } from 'src/constants/defaultNetworks';
import { useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import { Networks } from 'src/types/networks';
import { decryptMnemonic } from 'src/utils/mnemonicCrypto';
import { getOfflineSigner as getOfflineSignerFromMnemonic } from 'src/utils/offlineSigner';
import { getEncryptedMnemonic } from 'src/utils/utils';

const MNEMONIC_AUTO_CLEAR_MS = 15 * 60 * 1000; // 15 minutes

type SignerClientContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly signer: Option<OfflineSigner>;
  readonly signerReady: boolean;
  readonly getSignClientByChainId: (
    chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
  ) => Promise<Option<SigningCyberClient>>;
  setSigner: (signer: Option<OfflineSigner>) => void;
  activateWalletSigner: (signer: OfflineSigner, mnemonic: string) => void;
  unlockWallet: (password: string) => Promise<void>;
  getSignerForChain: (chainId: string) => Promise<Option<OfflineSigner>>;
};

async function createClient(signer: OfflineSigner): Promise<SigningCyberClient> {
  const rpcUrl = await getHealthyRpcUrl(CHAIN_ID, RPC_URL);
  const client = await SigningCyberClient.connectWithSigner(rpcUrl, signer);
  return client;
}

const SignerClientContext = React.createContext<SignerClientContextType>({
  signer: undefined,
  signingClient: undefined,
  signerReady: false,
  setSigner: () => {},
  activateWalletSigner: () => {},
  unlockWallet: async () => {},
  getSignerForChain: async () => undefined,
  getSignClientByChainId: async () => undefined,
});

export function useSigningClient() {
  const signingClient = useContext(SignerClientContext);
  return signingClient;
}

function SigningClientProvider({ children }: { children: React.ReactNode }) {
  const { defaultAccount } = useAppSelector((state) => state.pocket);
  const [signer, setSigner] = useState<SignerClientContextType['signer']>();
  const [signerReady, setSignerReady] = useState(false);
  const [signingClient, setSigningClient] = useState<SignerClientContextType['signingClient']>();
  const mnemonicRef = useRef<string | null>(null);
  const mnemonicTimerRef = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    (async () => {
      const address = signer ? (await signer.getAccounts())[0].address : undefined;

      setSignerReady(
        Boolean(address) &&
          Boolean(defaultAccount.account) &&
          address === defaultAccount.account?.cyber.bech32
      );
    })();
  }, [defaultAccount, signer]);

  // Rebuild signingClient whenever signer changes
  useEffect(() => {
    if (signer) {
      createClient(signer).then(setSigningClient);
    } else {
      setSigningClient(undefined);
    }
  }, [signer]);

  const isWalletAccount = defaultAccount.account?.cyber?.keys === 'wallet';
  const isLedgerAccount = defaultAccount.account?.cyber?.keys === 'ledger';

  const getSignClientByChainId = useCallback(
    async (chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) => {
      let offlineSigner: Option<OfflineSigner>;
      if (isWalletAccount && mnemonicRef.current) {
        offlineSigner = await getOfflineSignerFromMnemonic(mnemonicRef.current, chainId);
      }

      if (!offlineSigner) {
        return undefined;
      }

      const { RPC_URL: _RPC_URL } = defaultNetworks[chainId];
      const rpcUrl = await getHealthyRpcUrl(chainId, _RPC_URL);

      return SigningCyberClient.connectWithSigner(rpcUrl, offlineSigner);
    },
    [isWalletAccount]
  );

  const setMnemonicWithAutoClear = useCallback((mnemonic: string | null) => {
    if (mnemonicTimerRef.current) {
      clearTimeout(mnemonicTimerRef.current);
    }
    mnemonicRef.current = mnemonic;
    if (mnemonic) {
      mnemonicTimerRef.current = setTimeout(() => {
        mnemonicRef.current = null;
        setSigner(undefined);
        window.dispatchEvent(new CustomEvent('__cyb_wallet_locked'));
      }, MNEMONIC_AUTO_CLEAR_MS);
    }
  }, []);

  // Clear mnemonic on unmount
  useEffect(() => {
    return () => {
      mnemonicRef.current = null;
      if (mnemonicTimerRef.current) clearTimeout(mnemonicTimerRef.current);
    };
  }, []);

  // Auto-lock when tab becomes hidden — skip for Ledger (device IS security)
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.hidden && mnemonicRef.current && !isLedgerAccount) {
        mnemonicRef.current = null;
        if (mnemonicTimerRef.current) clearTimeout(mnemonicTimerRef.current);
        setSigner(undefined);
        window.dispatchEvent(new CustomEvent('__cyb_wallet_locked'));
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [isLedgerAccount]);

  const activateWalletSigner = useCallback(
    (offlineSigner: OfflineSigner, mnemonic: string) => {
      setMnemonicWithAutoClear(mnemonic);
      setSigner(offlineSigner);
    },
    [setMnemonicWithAutoClear]
  );

  const unlockWallet = useCallback(
    async (password: string) => {
      const address = defaultAccount.account?.cyber.bech32;
      if (!address) throw new Error('No active account');

      const encrypted = getEncryptedMnemonic(address);
      if (!encrypted) throw new Error('No encrypted mnemonic found');

      const mnemonic = await decryptMnemonic(encrypted, password);
      const offlineSigner = await getOfflineSignerFromMnemonic(mnemonic);

      // Verify decrypted mnemonic derives to the expected address
      const [account] = await offlineSigner.getAccounts();
      if (account.address !== address) {
        throw new Error('Decrypted mnemonic does not match expected address');
      }

      setMnemonicWithAutoClear(mnemonic);
      setSigner(offlineSigner);
    },
    [defaultAccount, setMnemonicWithAutoClear]
  );

  const getSignerForChain = useCallback(
    async (chainId: string): Promise<Option<OfflineSigner>> => {
      if (mnemonicRef.current) {
        return getOfflineSignerFromMnemonic(mnemonicRef.current, chainId);
      }
      return undefined;
    },
    []
  );

  const value = useMemo(
    () => ({
      signer,
      setSigner,
      activateWalletSigner,
      signingClient,
      signerReady,
      unlockWallet,
      getSignerForChain,
      getSignClientByChainId,
    }),
    [signer, signingClient, signerReady, setSigner, activateWalletSigner, unlockWallet, getSignerForChain, getSignClientByChainId]
  );

  return <SignerClientContext.Provider value={value}>{children}</SignerClientContext.Provider>;
}

export default SigningClientProvider;
