import { SigningCyberClient } from '@cybercongress/cyber-js';
import { OfflineSigner } from '@cybercongress/cyber-js/build/signingcyberclient';
import { Keplr } from '@keplr-wallet/types';
import _ from 'lodash';
import React, { useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import defaultNetworks, { getHealthyRpcUrl } from 'src/constants/defaultNetworks';
import usePrevious from 'src/hooks/usePrevious';
import { addAddressPocket, setDefaultAccount } from 'src/redux/features/pocket';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import { Networks } from 'src/types/networks';
import { decryptMnemonic, encryptMnemonic, getTauriDeviceKey } from 'src/utils/mnemonicCrypto';
import {
  connectLedger as connectLedgerDevice,
  ReconnectingLedgerSigner,
  createLedgerSigner,
  closeTransport,
  checkTransportHealth,
} from 'src/utils/ledgerSigner';
import networkListIbc from 'src/utils/networkListIbc';
import { getOfflineSigner as getOfflineSignerFromMnemonic, getOfflineSignerFromPrivateKey } from 'src/utils/offlineSigner';
import { getEncryptedMnemonic, setEncryptedMnemonic } from 'src/utils/utils';

const MNEMONIC_AUTO_CLEAR_MS = 15 * 60 * 1000; // 15 minutes

type SignerClientContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly signer: Option<OfflineSigner>;
  readonly signerReady: boolean;
  readonly getSignClientByChainId: (
    chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
  ) => Promise<Option<SigningCyberClient>>;
  initSigner: () => void;
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
  getSignClientByChainId: () => {},
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

  const initSigner = useCallback(async () => {
    const offlineSigner = await getOfflineSigner(CHAIN_ID);

    if (!offlineSigner) {
      return;
    }

    const clientJs = await createClient(offlineSigner);

    setSigner(offlineSigner);
    setSigningClient(clientJs);
  }, [getOfflineSigner]);

  useEffect(() => {
    (async () => {
      const windowKeplr = await getKeplr();
      if (windowKeplr) {
        initSigner();
      }
    })();
  }, [initSigner]);

  useEffect(() => {
    (async () => {
      const keys = defaultAccount.account?.cyber?.keys;
      const bech32 = defaultAccount.account?.cyber?.bech32;
      if ((keys !== 'wallet' && keys !== 'private-key') || !bech32) return;

      const isPrivateKey = keys === 'private-key';

      // On web, auto-switch only works if wallet is already unlocked (mnemonicRef)
      // On Tauri, decrypt with device key
      let secret: string | null = mnemonicRef.current;

      if (!secret && process.env.IS_TAURI) {
        const encrypted = getEncryptedMnemonic(bech32);
        if (encrypted) {
          try {
            secret = await decryptMnemonic(encrypted, getTauriDeviceKey());
          } catch {
            console.warn('[Signer] Failed to decrypt key for account switch');
            return;
          }
        }
      }

      if (!secret) return;

      try {
        const localSigner = isPrivateKey
          ? await getOfflineSignerFromPrivateKey(secret)
          : await getOfflineSignerFromMnemonic(secret);
        const [account] = await localSigner.getAccounts();
        if (account.address !== bech32) {
          console.warn('[Signer] Key derives different address, skipping');
          return;
        }
        const clientJs = await createClient(localSigner);
        setSigner(localSigner);
        setSigningClient(clientJs);
        console.log('[Signer] Switched to local account:', bech32);
      } catch (e) {
        console.error('[Signer] Failed to switch to local account:', e);
      }
    })();
  }, [defaultAccount]);

  const getSignClientByChainId = useCallback(
    async (chainId: Networks.BOSTROM | Networks.SPACE_PUSSY) => {
      const offlineSigner = await getOfflineSigner(chainId);

      if (!offlineSigner) {
        return undefined;
      }

      const { RPC_URL: _RPC_URL } = defaultNetworks[chainId];
      const rpcUrl = await getHealthyRpcUrl(chainId, _RPC_URL);

      return SigningCyberClient.connectWithSigner(rpcUrl, offlineSigner);
    },
    [isWalletAccount, isLedgerAccount]
  );

  const setMnemonicWithAutoClear = useCallback((mnemonic: string | null) => {
    if (mnemonicTimerRef.current) {
      clearTimeout(mnemonicTimerRef.current);
    }
    mnemonicRef.current = mnemonic;
  }, []);

  // Clear mnemonic on unmount
  useEffect(() => {
    return () => {
      mnemonicRef.current = null;
      if (mnemonicTimerRef.current) clearTimeout(mnemonicTimerRef.current);
    };
  }, []);

  // Auto-lock disabled — wallet stays unlocked until device locks

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
      if (!address) throw new Error('Select an account in Keys before signing');

      const isPrivateKeyAccount = defaultAccount.account?.cyber.keys === 'private-key';

      const encrypted = getEncryptedMnemonic(address);
      if (!encrypted) throw new Error('Wallet data not found. Re-import your key');

      const secret = await decryptMnemonic(encrypted, password);
      const offlineSigner = isPrivateKeyAccount
        ? await getOfflineSignerFromPrivateKey(secret)
        : await getOfflineSignerFromMnemonic(secret);

      // Verify decrypted secret derives to the expected address
      const [account] = await offlineSigner.getAccounts();
      if (account.address !== address) {
        throw new Error('Key does not match this account. Check your backup');
      }

      setMnemonicWithAutoClear(secret);
      setSigner(offlineSigner);
    },
    [defaultAccount, setMnemonicWithAutoClear]
  );

  // Connect Ledger — requires user gesture (WebUSB)
  const connectLedgerFn = useCallback(async () => {
    const { signer: ledgerSigner, address, pubkey } = await connectLedgerDevice();
    setSigner(ledgerSigner);
    return { address, pubkey };
  }, []);

  // Reconnect Ledger — for when signer was lost (page refresh / device sleep)
  const reconnectLedger = useCallback(async () => {
    if (!isLedgerAccount) return;

    // Validate device first with a raw signer
    const rawSigner = await createLedgerSigner();
    const expectedAddress = defaultAccount.account?.cyber?.bech32;
    if (expectedAddress) {
      const [account] = await rawSigner.getAccounts();
      if (account.address !== expectedAddress) {
        throw new Error(
          'This Ledger has a different address. Is it the correct device?'
        );
      }
    }

    // Use ReconnectingLedgerSigner for long-lived use — survives sleep
    const [account] = await rawSigner.getAccounts();
    const reconnectingSigner = new ReconnectingLedgerSigner('bostrom', [account]);
    setSigner(reconnectingSigner);
  }, [isLedgerAccount, defaultAccount]);

  // Ledger health monitoring — detect sleep / disconnect
  useEffect(() => {
    if (!isLedgerAccount || !signer) return;

    const HEALTH_INTERVAL_MS = 30_000; // 30 seconds

    const check = async () => {
      const healthy = await checkTransportHealth();
      if (!healthy) {
        setSigner(undefined);
        window.dispatchEvent(new CustomEvent('__cyb_ledger_disconnected'));
      }
    };

    const interval = setInterval(check, HEALTH_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [isLedgerAccount, signer]);

  // Close transport on unmount and on page unload
  useEffect(() => {
    const handleBeforeUnload = () => {
      closeTransport();
    };
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
      closeTransport();
    };
  }, []);

  const getSignerForChain = useCallback(
    async (chainId: string): Promise<Option<OfflineSigner>> => {
      if (isLedgerAccount) {
        const network = networkListIbc[chainId];
        const prefix = network?.prefix;
        if (prefix) {
          return createLedgerSigner(prefix);
        }
        return undefined;
      }
      if (mnemonicRef.current) {
        return getOfflineSignerFromMnemonic(mnemonicRef.current, chainId);
      }
      return undefined;
    },
    [isLedgerAccount]
  );

  const value = useMemo(
    () => ({
      initSigner,
      signer,
      signingClient,
      signerReady,
      getSignClientByChainId,
    }),
    [signer, signingClient, signerReady, initSigner, getSignClientByChainId]
  );

  return <SignerClientContext.Provider value={value}>{children}</SignerClientContext.Provider>;
}

export default SigningClientProvider;
