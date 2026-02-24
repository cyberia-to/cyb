export enum Networks {
  BOSTROM = 'bostrom',
  LOCAL_BOSTROM = 'localbostrom',
  SPACE_PUSSY = 'space-pussy',
  ETH = 'eth',
  OSMO = 'osmo',
  TERRA = 'terra',
  COSMOS = 'cosmoshub-4',
}

export type NetworkConfig = {
  CHAIN_ID: Networks;
  BASE_DENOM: string;
  DENOM_LIQUID: string;
  RPC_URL: string;
  LCD_URL: string;
  WEBSOCKET_URL: string;
  INDEX_HTTPS: string;
  INDEX_WEBSOCKET: string;
  BECH32_PREFIX: string;
  MEMO_KEPLR: string;
};

export type NetworksList = {
  [key in Networks]: NetworkConfig;
};
