import { Keplr } from '@keplr-wallet/types';
import { BASE_DENOM, CHAIN_ID, LCD_URL, RPC_URL } from 'src/constants/config';

export const getKeplr = async (): Promise<Keplr | undefined> => {
  if (window.keplr) {
    return window.keplr;
  }

  if (document.readyState === 'complete') {
    return window.keplr;
  }

  return new Promise((resolve) => {
    const documentStateChange = (event: Event) => {
      if (event.target && (event.target as Document).readyState === 'complete') {
        resolve(window.keplr);
        document.removeEventListener('readystatechange', documentStateChange);
      }
    };

    document.addEventListener('readystatechange', documentStateChange);
  });
};

const configKeplr = (prefix: string) => {
  return {
    // Chain-id of the Cosmos SDK chain.
    chainId: CHAIN_ID,
    // The name of the chain to be displayed to the user.
    chainName: CHAIN_ID,
    // RPC endpoint of the chain.
    rpc: RPC_URL,
    rest: LCD_URL,
    stakeCurrency: {
      coinDenom: BASE_DENOM.toUpperCase(),
      coinMinimalDenom: BASE_DENOM,
      coinDecimals: 0,
    },
    bip44: {
      // You can only set the coin type of BIP44.
      // 'Purpose' is fixed to 44.
      coinType: 118,
    },
    bech32Config: {
      bech32PrefixAccAddr: prefix,
      bech32PrefixAccPub: `${prefix}pub`,
      bech32PrefixValAddr: `${prefix}valoper`,
      bech32PrefixValPub: `${prefix}valoperpub`,
      bech32PrefixConsAddr: `${prefix}valcons`,
      bech32PrefixConsPub: `${prefix}valconspub`,
    },
    currencies: [
      {
        coinDenom: BASE_DENOM.toUpperCase(),
        coinMinimalDenom: BASE_DENOM,
        coinDecimals: 0,
      },
      {
        coinDenom: 'L',
        coinMinimalDenom: 'liquidpussy',
        coinDecimals: 0,
      },
      {
        coinDenom: 'V',
        coinMinimalDenom: 'millivolt',
        coinDecimals: 3,
      },
      {
        coinDenom: 'A',
        coinMinimalDenom: 'milliampere',
        coinDecimals: 3,
      },
    ],
    // List of coin/tokens used as a fee token in this chain.
    feeCurrencies: [
      {
        // Coin denomination to be displayed to the user.
        coinDenom: BASE_DENOM.toUpperCase(),
        // Actual denom (i.e. uatom, uscrt) used by the blockchain.
        coinMinimalDenom: BASE_DENOM,
        // # of decimal points to convert minimal denomination to user-facing denomination.
        coinDecimals: 0,
      },
    ],
    features: ['ibc-transfer'],
  };
};

export default configKeplr;
