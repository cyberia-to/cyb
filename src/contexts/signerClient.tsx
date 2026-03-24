import { SigningCyberClient } from '@cybercongress/cyber-js';
import { OfflineSigner } from '@cybercongress/cyber-js/build/signingcyberclient';
import React, { useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import defaultNetworks, { getHealthyRpcUrl } from 'src/constants/defaultNetworks';
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
import { getOfflineSigner as getOfflineSignerFromMnemonic } from 'src/utils/offlineSigner';
import { getEncryptedMnemonic, setEncryptedMnemonic } from 'src/utils/utils';

const MNEMONIC_AUTO_CLEAR_MS = 15 * 60 * 1000; // 15 minutes

type SignerClientContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly signer: Option<OfflineSigner>;
  readonly signerReady: boolean;
  readonly isLedgerAccount: boolean;
  readonly getSignClientByChainId: (
    chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
  ) => Promise<Option<SigningCyberClient>>;
  setSigner: (signer: Option<OfflineSigner>) => void;
  activateWalletSigner: (signer: OfflineSigner, mnemonic: string) => void;
  unlockWallet: (password: string) => Promise<void>;
  connectLedger: () => Promise<{ address: string; pubkey: Uint8Array }>;
  reconnectLedger: () => Promise<void>;
  getSignerForChain: (chainId: string) => Promise<Option<OfflineSigner>>;
};

async function createClient(signer: OfflineSigner): Promise<SigningCyberClient> {
  const rpcUrl = await getHealthyRpcUrl(CHAIN_ID, RPC_URL);
  const client = await SigningCyberClient.connectWithSigner(rpcUrl, signer);
  return client;
}

export const SignerClientContext = React.createContext<SignerClientContextType>({
  signer: undefined,
  signingClient: undefined,
  signerReady: false,
  isLedgerAccount: false,
  setSigner: () => {},
  activateWalletSigner: () => {},
  unlockWallet: async () => {},
  connectLedger: async () => ({ address: '', pubkey: new Uint8Array() }),
  reconnectLedger: async () => {},
  getSignerForChain: async () => undefined,
  getSignClientByChainId: async () => undefined,
});

export function useSigningClient() {
  return useContext(SignerClientContext);
}

function SigningClientProvider({ children }: { children: React.ReactNode }) {
  const dispatch = useAppDispatch();
  const { defaultAccount } = useAppSelector((state) => state.pocket);
  const [signer, setSigner] = useState<SignerClientContextType['signer']>();
  const [signerReady, setSignerReady] = useState(false);
  const [signingClient, setSigningClient] = useState<SignerClientContextType['signingClient']>();
  const mnemonicRef = useRef<string | null>(null);
  const mnemonicTimerRef = useRef<ReturnType<typeof setTimeout>>();

  const isWalletAccount = defaultAccount.account?.cyber?.keys === 'wallet';
  const isLedgerAccount = defaultAccount.account?.cyber?.keys === 'ledger';

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
      createClient(signer)
        .then(setSigningClient)
        .catch((err) => {
          console.error('Failed to create signing client:', err);
          setSigningClient(undefined);
        });
    } else {
      setSigningClient(undefined);
    }
  }, [signer]);

  // Ledger disconnect detection on page refresh
  useEffect(() => {
    if (isLedgerAccount && !signer) {
      window.dispatchEvent(new CustomEvent('__cyb_ledger_disconnected'));
    }
  }, [isLedgerAccount, signer]);

  // Tauri or web without Keplr: auto-generate mnemonic on first launch,
  // restore saved mnemonic on subsequent launches.
  // addAddressPocket deduplicates — won't re-register or override default account.
  useEffect(() => {
    (async () => {
      if (!process.env.IS_TAURI) return;

      try {
        let mnemonic: string | null = null;
        let walletSource = 'existing';
        let accountName = 'Account 1';

        // Check bootstrap.json FIRST — it's an explicit user action (download from web)
        // and should override any existing wallet
        if (process.env.IS_TAURI) {
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            console.log('[Bootstrap] Invoking read_bootstrap...');
            const bootstrap = await invoke('read_bootstrap') as { mnemonic?: string; referrer?: string; name?: string } | null;
            console.log('[Bootstrap] read_bootstrap result:', bootstrap ? 'found' : 'null');
            if (bootstrap?.mnemonic) {
              mnemonic = bootstrap.mnemonic;
              walletSource = 'cyb-boot';
              if (bootstrap.name) {
                accountName = bootstrap.name;
              }
              console.log('[Bootstrap] Mnemonic imported from cyb-boot, name:', accountName);
              if (bootstrap.referrer) {
                const { saveReferrer } = await import('src/pages/Mining/components/ReferralSection');
                saveReferrer(bootstrap.referrer);
                console.log('[Bootstrap] Referrer saved:', bootstrap.referrer);
              } else {
                console.log('[Bootstrap] No referrer in bootstrap');
              }
            }
          } catch (err) {
            console.log('[Bootstrap] No bootstrap.json found (normal first launch):', err);
          }
        }

        if (!mnemonic) {
          const deviceKey = getTauriDeviceKey();

          // Migrate legacy plaintext mnemonic (old Tauri key without address suffix)
          const legacyKey = 'cyb:mnemonic';
          const legacyMnemonic = localStorage.getItem(legacyKey);
          if (legacyMnemonic && legacyMnemonic.split(' ').length >= 12) {
            mnemonic = legacyMnemonic;
            walletSource = 'migrated';
            localStorage.removeItem(legacyKey);
            console.log('[Bootstrap] Migrated legacy plaintext mnemonic');
          }

          // Try to restore from encrypted storage
          if (!mnemonic) {
            for (let i = 0; i < localStorage.length; i++) {
              const key = localStorage.key(i);
              if (key?.startsWith('cyb:mnemonic:')) {
                const encrypted = localStorage.getItem(key);
                if (encrypted) {
                  try {
                    mnemonic = await decryptMnemonic(encrypted, deviceKey);
                    console.log('[Bootstrap] Restored encrypted wallet from storage');
                    break;
                  } catch {
                    // Not decryptable with device key, skip
                  }
                }
              }
            }
          }
        }

        if (!mnemonic) {
          const { generateMnemonic } = await import('src/utils/offlineSigner');
          mnemonic = await generateMnemonic();
          walletSource = 'generated';
          console.log('[Bootstrap] Auto-generated new wallet');
        }

        const mnemonicSigner = await getOfflineSignerFromMnemonic(mnemonic);
        const mnemonicAccounts = await mnemonicSigner.getAccounts();
        const { address } = mnemonicAccounts[0];
        const pk = Buffer.from(mnemonicAccounts[0].pubkey).toString('hex');

        console.log(`[Bootstrap] Wallet active: ${address} (source: ${walletSource}, name: ${accountName})`);

        // Store mnemonic encrypted with device key
        const deviceKey = getTauriDeviceKey();
        const encrypted = await encryptMnemonic(mnemonic, deviceKey);
        setEncryptedMnemonic(encrypted, address);

        // Register account (deduplicates if already exists)
        dispatch(
          addAddressPocket({
            bech32: address,
            keys: 'wallet',
            pk,
            name: accountName,
          })
        );

        // Force set as active — addAddressPocket skips this if initPocket already ran
        if (walletSource === 'cyb-boot' || walletSource === 'generated') {
          console.log(`[Bootstrap] Setting ${address} as default account (${accountName})`);
          dispatch(
            setDefaultAccount({
              name: accountName,
              account: { cyber: { bech32: address, keys: 'wallet', pk, name: accountName } },
            })
          );
        }

        const clientJs = await createClient(mnemonicSigner);
        setSigner(mnemonicSigner);
        setSigningClient(clientJs);
        setSignerReady(true);
        console.log(`[Bootstrap] Signing client ready (${walletSource})`);
      } catch (e) {
        console.error('[Bootstrap] Failed to init signer client:', e);
      }
    })();
  }, []);

  // Auto-switch signer when defaultAccount changes to a local wallet
  useEffect(() => {
    (async () => {
      const keys = defaultAccount.account?.cyber?.keys;
      const bech32 = defaultAccount.account?.cyber?.bech32;
      if (keys !== 'wallet' || !bech32) return;

      // On web, auto-switch only works if wallet is already unlocked (mnemonicRef)
      // On Tauri, decrypt with device key
      let mnemonic: string | null = mnemonicRef.current;

      if (!mnemonic && process.env.IS_TAURI) {
        const encrypted = getEncryptedMnemonic(bech32);
        if (encrypted) {
          try {
            mnemonic = await decryptMnemonic(encrypted, getTauriDeviceKey());
          } catch {
            console.warn('[Signer] Failed to decrypt mnemonic for account switch');
            return;
          }
        }
      }

      if (!mnemonic) return;

      try {
        const localSigner = await getOfflineSignerFromMnemonic(mnemonic);
        const [account] = await localSigner.getAccounts();
        if (account.address !== bech32) {
          console.warn('[Signer] Mnemonic derives different address, skipping');
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
      let offlineSigner: Option<OfflineSigner>;

      if (isLedgerAccount) {
        const { BECH32_PREFIX: prefix } = defaultNetworks[chainId];
        offlineSigner = await createLedgerSigner(prefix);
      } else if (isWalletAccount && mnemonicRef.current) {
        offlineSigner = await getOfflineSignerFromMnemonic(mnemonicRef.current, chainId);
      }

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

      const encrypted = getEncryptedMnemonic(address);
      if (!encrypted) throw new Error('Wallet data not found. Re-import your seed phrase');

      const mnemonic = await decryptMnemonic(encrypted, password);
      const offlineSigner = await getOfflineSignerFromMnemonic(mnemonic);

      // Verify decrypted mnemonic derives to the expected address
      const [account] = await offlineSigner.getAccounts();
      if (account.address !== address) {
        throw new Error('Seed phrase does not match this account. Check your backup');
      }

      setMnemonicWithAutoClear(mnemonic);
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
      signer,
      setSigner,
      activateWalletSigner,
      signingClient,
      signerReady,
      isLedgerAccount,
      unlockWallet,
      connectLedger: connectLedgerFn,
      reconnectLedger,
      getSignerForChain,
      getSignClientByChainId,
    }),
    [signer, signingClient, signerReady, isLedgerAccount, setSigner, activateWalletSigner, unlockWallet, connectLedgerFn, reconnectLedger, getSignerForChain, getSignClientByChainId]
  );

  return <SignerClientContext.Provider value={value}>{children}</SignerClientContext.Provider>;
}

export default SigningClientProvider;
