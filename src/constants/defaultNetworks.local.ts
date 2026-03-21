import { NetworkConfig, Networks } from 'src/types/networks';

type NetworksList = {
  [key in Networks.BOSTROM | Networks.SPACE_PUSSY]: NetworkConfig;
};

const defaultNetworks: NetworksList = {
  bostrom: {
    CHAIN_ID: Networks.BOSTROM,
    BASE_DENOM: 'boot',
    DENOM_LIQUID: 'hydrogen',
    RPC_URL: 'http://localhost:26657', //'https://rpc.bostrom.cybernode.ai',
    LCD_URL: 'http://localhost:1317/', //'https://lcd.bostrom.cybernode.ai',
    WEBSOCKET_URL: 'ws://localhost:26657', //'wss://rpc.bostrom.cybernode.ai/websocket',
    INDEX_HTTPS: 'http://localhost:8090/v1/graphql', //'https://index.bostrom.cybernode.ai/v1/graphql',
    INDEX_WEBSOCKET: 'ws://localhost:8090/v1/graphql', // 'wss://index.bostrom.cybernode.ai/v1/graphql',
    BECH32_PREFIX: 'bostrom',
    MEMO: '[bostrom] cyb.ai',
  },
  'space-pussy': {
    CHAIN_ID: Networks.SPACE_PUSSY,
    BASE_DENOM: 'pussy',
    DENOM_LIQUID: 'liquidpussy',
    RPC_URL: 'https://rpc.space-pussy.cybernode.ai/',
    LCD_URL: 'https://lcd.space-pussy.cybernode.ai',
    WEBSOCKET_URL: 'wss://rpc.space-pussy.cybernode.ai/websocket',
    INDEX_HTTPS: 'https://index.space-pussy.cybernode.ai/v1/graphql',
    INDEX_WEBSOCKET: 'wss://index.space-pussy.cybernode.ai/v1/graphql',
    BECH32_PREFIX: 'pussy',
    MEMO: '[space-pussy] cyb.ai',
  },
};

export default defaultNetworks;
