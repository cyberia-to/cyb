import { SigningCyberClient } from '@cybercongress/cyber-js';
import { OfflineSigner } from '@cybercongress/cyber-js/build/signingcyberclient';
import { Keplr } from '@keplr-wallet/types';
import _ from 'lodash';
import React, { useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import defaultNetworks, { getHealthyRpcUrl } from 'src/constants/defaultNetworks';
import usePrevious from 'src/hooks/usePrevious';
import { addAddressPocket, setDefaultAccount } from 'src/redux/features/pocket';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import { Networks } from 'src/types/networks';
import configKeplr, { getKeplr } from 'src/utils/keplrUtils';
import { decryptMnemonic } from 'src/utils/mnemonicCrypto';
import { getOfflineSigner as getOfflineSignerFromMnemonic } from 'src/utils/offlineSigner';
import { accountsKeplr, getEncryptedMnemonic } from 'src/utils/utils';

const MNEMONIC_AUTO_CLEAR_MS = 15 * 60 * 1000; // 15 minutes

type SignerClientContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly signer: Option<OfflineSigner>;
  readonly signerReady: boolean;
  readonly getSignClientByChainId: (
    chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
  ) => Promise<Option<SigningCyberClient>>;
  initSigner: () => void;
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
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  initSigner: () => {},
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
  const { defaultAccount, accounts } = useAppSelector((state) => state.pocket);
  const dispatch = useAppDispatch();
  const [signer, setSigner] = useState<SignerClientContextType['signer']>();
  const [signerReady, setSignerReady] = useState(false);
  const [signingClient, setSigningClient] = useState<SignerClientContextType['signingClient']>();
  const mnemonicRef = useRef<string | null>(null);
  const mnemonicTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const prevAccounts = usePrevious(accounts);

  const selectAddress = useCallback(
    async (keplr: Keplr) => {
      if (!accounts || _.isEqual(prevAccounts, accounts)) {
        return;
      }
      const keyInfo = await keplr.getKey(CHAIN_ID);

      const findAccount = Object.keys(accounts).find((key) => {
        if (accounts[key].cyber.bech32 === keyInfo.bech32Address) {
          return key;
        }

        return undefined;
      });

      if (findAccount) {
        dispatch(setDefaultAccount({ name: findAccount }));
      } else {
        dispatch(addAddressPocket(accountsKeplr(keyInfo)));
      }
    },
    [accounts, prevAccounts, dispatch]
  );

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

  const getOfflineSigner = useCallback(
    async (chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) => {
      const windowKeplr = await getKeplr();

      if (!windowKeplr || !windowKeplr.experimentalSuggestChain) {
        return undefined;
      }

      const { CHAIN_ID: _CHAIN_ID, BECH32_PREFIX: _BECH32_PREFIX } = defaultNetworks[chainId];

      if (CHAIN_ID === _CHAIN_ID) {
        selectAddress(windowKeplr);
      }

      windowKeplr.defaultOptions = {
        sign: {
          preferNoSetFee: true,
        },
      };
      await windowKeplr.experimentalSuggestChain(configKeplr(_BECH32_PREFIX));
      await windowKeplr.enable(CHAIN_ID);
      const offlineSigner = await windowKeplr.getOfflineSignerAuto(_CHAIN_ID);

      return offlineSigner;
    },
    [selectAddress]
  );

  // Rebuild signingClient whenever signer changes
  useEffect(() => {
    if (signer) {
      createClient(signer).then(setSigningClient);
    } else {
      setSigningClient(undefined);
    }
  }, [signer]);

  const initSigner = useCallback(async () => {
    const offlineSigner = await getOfflineSigner(CHAIN_ID);

    if (!offlineSigner) {
      return;
    }

    setSigner(offlineSigner);
  }, [getOfflineSigner]);

  // Keplr auto-init disabled — wallet uses mnemonic signer, Keplr only for IBC
  const isWalletAccount = defaultAccount.account?.cyber?.keys === 'wallet';

  // keplr_keystorechange listener — only for keplr accounts
  useEffect(() => {
    if (isWalletAccount) {
      return;
    }

    const handleKeystoreChange = () => {
      initSigner();
    };

    window.addEventListener('keplr_keystorechange', handleKeystoreChange);
    return () => {
      window.removeEventListener('keplr_keystorechange', handleKeystoreChange);
    };
  }, [initSigner, isWalletAccount]);

  const getSignClientByChainId = useCallback(
    async (chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) => {
      // Use mnemonic signer for wallet accounts, Keplr only for keplr accounts
      let offlineSigner: Option<OfflineSigner>;
      if (isWalletAccount && mnemonicRef.current) {
        offlineSigner = await getOfflineSignerFromMnemonic(mnemonicRef.current, chainId);
      } else if (!isWalletAccount) {
        offlineSigner = await getOfflineSigner(chainId);
      }

      if (!offlineSigner) {
        return undefined;
      }

      const { RPC_URL: _RPC_URL } = defaultNetworks[chainId];
      const rpcUrl = await getHealthyRpcUrl(chainId, _RPC_URL);

      return SigningCyberClient.connectWithSigner(rpcUrl, offlineSigner);
    },
    [getOfflineSigner, isWalletAccount]
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

  // Auto-lock when tab becomes hidden (user switches tab / minimizes / locks screen)
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.hidden && mnemonicRef.current) {
        mnemonicRef.current = null;
        if (mnemonicTimerRef.current) clearTimeout(mnemonicTimerRef.current);
        setSigner(undefined);
        window.dispatchEvent(new CustomEvent('__cyb_wallet_locked'));
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, []);

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
      // Fall back to Keplr only for non-wallet accounts
      if (!isWalletAccount) {
        const windowKeplr = await getKeplr();
        if (windowKeplr) {
          await windowKeplr.enable(chainId);
          return windowKeplr.getOfflineSignerAuto(chainId);
        }
      }
      return undefined;
    },
    [isWalletAccount]
  );

  const value = useMemo(
    () => ({
      initSigner,
      signer,
      setSigner,
      activateWalletSigner,
      signingClient,
      signerReady,
      unlockWallet,
      getSignerForChain,
      getSignClientByChainId,
    }),
    [signer, signingClient, signerReady, initSigner, setSigner, activateWalletSigner, unlockWallet, getSignerForChain, getSignClientByChainId]
  );

  return <SignerClientContext.Provider value={value}>{children}</SignerClientContext.Provider>;
}

export default SigningClientProvider;
