import { NetworkConfig, Networks } from 'src/types/networks';

type NetworksList = {
  [key in Networks]: NetworkConfig;
};

// Backup RPC endpoints for failover
export const RPC_ENDPOINTS: Record<string, string[]> = {
  [Networks.BOSTROM]: [
    'https://rpc.bostrom.cybernode.ai',
    'https://rpc-cyber-ia.cosmosia.notional.ventures',
    'https://rpc.cyber.bronbro.io',
  ],
  [Networks.SPACE_PUSSY]: ['https://rpc.space-pussy.cybernode.ai'],
};

/**
 * Try RPC endpoints in order, return the first one that responds.
 * Caches the working endpoint for the session.
 */
const rpcCache = new Map<string, string>();

export async function getHealthyRpcUrl(chainId: string, defaultUrl: string): Promise<string> {
  const cached = rpcCache.get(chainId);
  if (cached) {
    return cached;
  }

  const endpoints = RPC_ENDPOINTS[chainId] || [defaultUrl];

  for (const endpoint of endpoints) {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);
      // eslint-disable-next-line no-await-in-loop
      const response = await fetch(`${endpoint}/status`, {
        signal: controller.signal,
      });
      clearTimeout(timeoutId);

      if (response.ok) {
        rpcCache.set(chainId, endpoint);
        return endpoint;
      }
    } catch {
      // try next endpoint
    }
  }

  // Fall back to default if nothing responds
  return defaultUrl;
}

const defaultNetworks: NetworksList = {
  bostrom: {
    CHAIN_ID: Networks.BOSTROM,
    BASE_DENOM: 'boot',
    DENOM_LIQUID: 'hydrogen',
    RPC_URL: 'https://rpc.bostrom.cybernode.ai',
    LCD_URL: 'https://lcd.bostrom.cybernode.ai',
    WEBSOCKET_URL: 'wss://rpc.bostrom.cybernode.ai/websocket',
    INDEX_HTTPS: 'https://index.bostrom.cybernode.ai/v1/graphql',
    INDEX_WEBSOCKET: 'wss://index.bostrom.cybernode.ai/v1/graphql',
    BECH32_PREFIX: 'bostrom',
    MEMO_KEPLR: '[bostrom] cyb.ai, using keplr',
  },
  localbostrom: {
    CHAIN_ID: Networks.LOCAL_BOSTROM,
    BASE_DENOM: 'boot',
    DENOM_LIQUID: 'hydrogen',
    RPC_URL: 'https://rpc.bostrom.moon.cybernode.ai',
    LCD_URL: 'https://lcd.bostrom.moon.cybernode.ai',
    WEBSOCKET_URL: 'wss://rpc.bostrom.moon.cybernode.ai/websocket',
    INDEX_HTTPS: 'https://index.bostrom.moon.cybernode.ai/v1/graphql',
    INDEX_WEBSOCKET: 'wss://index.bostrom.moon.cybernode.ai/v1/graphql',
    BECH32_PREFIX: 'bostrom',
    MEMO_KEPLR: '[bostrom] cyb.ai, using keplr',
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
    MEMO_KEPLR: '[space-pussy] cyb.ai, using keplr',
  },
};

export default defaultNetworks;
