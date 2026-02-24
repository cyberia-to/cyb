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
import configKeplr, { getKeplr } from 'src/utils/keplrUtils';
import { getOfflineSigner as getOfflineSignerFromMnemonic } from 'src/utils/offlineSigner';
import { accountsKeplr, getMnemonic, setMnemonic } from 'src/utils/utils';

type SignerClientContextType = {
  readonly signingClient: Option<SigningCyberClient>;
  readonly signer: Option<OfflineSigner>;
  readonly signerReady: boolean;
  readonly getSignClientByChainId: (
    chainId: Networks.BOSTROM | Networks.SPACE_PUSSY
  ) => Promise<Option<SigningCyberClient>>;
  initSigner: () => void;
  setSigner(signer: Option<OfflineSigner>): void;
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
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  initSigner: () => {},
  setSigner: () => {},
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
    const handleKeystoreChange = () => {
      initSigner();
    };

    window.addEventListener('keplr_keystorechange', handleKeystoreChange);
    return () => {
      window.removeEventListener('keplr_keystorechange', handleKeystoreChange);
    };
  }, [initSigner]);

  useEffect(() => {
    (async () => {
      if (process.env.IS_TAURI || !window.keplr) {
        console.log('Init signing client');
        try {
          let mnemonic = getMnemonic();

          // Auto-generate mnemonic for new users (mobile/Tauri)
          if (!mnemonic) {
            const { generateMnemonic } = await import(
              'src/utils/offlineSigner'
            );
            mnemonic = await generateMnemonic();
            setMnemonic(mnemonic);
            console.log('Auto-generated new wallet');
          }

          const mnemonicSigner = await getOfflineSignerFromMnemonic(mnemonic);
          const accounts = await mnemonicSigner.getAccounts();
          const { address } = accounts[0];
          const pk = Buffer.from(accounts[0].pubkey).toString('hex');

          // Register account in Redux so selectCurrentAddress works
          dispatch(
            addAddressPocket({
              bech32: address,
              keys: 'keplr',
              pk,
              name: 'Account 1',
            })
          );

          const clientJs = await createClient(mnemonicSigner);

          window.signer = mnemonicSigner;
          window.signingClient = clientJs;

          console.log('Signer is set');

          setSigner(mnemonicSigner);
          setSigningClient(clientJs);
          setSignerReady(true);
          console.log('Signing client init success');
        } catch (e) {
          console.error('Failed to init signer client:', e);
        }
      }
    })();
  }, []);

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
    [getOfflineSigner]
  );

  const value = useMemo(
    () => ({
      initSigner,
      signer,
      signingClient,
      signerReady,
      setSigner,
      getSignClientByChainId,
    }),
    [signer, signingClient, signerReady, initSigner, setSigner, getSignClientByChainId]
  );

  return <SignerClientContext.Provider value={value}>{children}</SignerClientContext.Provider>;
}

export default SigningClientProvider;
