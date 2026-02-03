import defaultNetworks from 'src/constants/defaultNetworks';

type NetworkIbc = {
  chainId: string;
  sourceChainId?: string;
  sourceChannelId?: string;
  destinationChainId?: string;
  destChannelId?: string;
  coinMinimalDenom: string;
  coinDecimals: number;
  denom: string;
  rpc: string;
  prefix: string;
  explorerUrlToTx: string;
};

enum NetworkChainID {
  bostrom = 'bostrom',
  osmosis = 'osmosis-1',
  cosmoshub = 'cosmoshub-4',
  spacePussy = 'space-pussy',
  juno = 'juno-1',
  gravityBridge = 'gravity-bridge-3',
  desmosMainnet = 'desmos-mainnet',
  evmos_9001 = 'evmos_9001-2',
  chihuahua = 'chihuahua-1',
  axelarDojo = 'axelar-dojo-1',
}

type NetworkList = { [key in NetworkChainID]: NetworkIbc };

const networkList: NetworkList = {
  bostrom: {
    chainId: defaultNetworks.bostrom.CHAIN_ID,
    rpc: defaultNetworks.bostrom.RPC_URL,
    coinMinimalDenom: defaultNetworks.bostrom.BASE_DENOM,
    coinDecimals: 0,
    denom: defaultNetworks.bostrom.BASE_DENOM.toUpperCase(),
    prefix: defaultNetworks.bostrom.BASE_DENOM,
    explorerUrlToTx: 'https://cyb.ai/network/bostrom/tx/{txHash}',
  },
  'osmosis-1': {
    chainId: 'osmosis-1',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-2',
    destinationChainId: 'osmosis-1',
    destChannelId: 'channel-95',
    coinMinimalDenom: 'uosmo',
    coinDecimals: 6,
    denom: 'OSMO',
    rpc: 'https://rpc.osmosis-1.bronbro.io',
    prefix: 'osmo',
    explorerUrlToTx: 'https://www.mintscan.io/osmosis/txs/{txHash}',
  },
  'cosmoshub-4': {
    chainId: 'cosmoshub-4',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-8',
    destinationChainId: 'cosmoshub-4',
    destChannelId: 'channel-341',
    coinMinimalDenom: 'uatom',
    coinDecimals: 6,
    denom: 'ATOM',
    rpc: 'https://rpc.cosmoshub-4.bronbro.io:443',
    prefix: 'cosmos',
    explorerUrlToTx: 'https://www.mintscan.io/cosmos/txs/{txHash}',
  },
  'space-pussy': {
    chainId: 'space-pussy',
    coinDecimals: 0,
    denom: 'PUSSY',
    coinMinimalDenom: 'pussy',
    rpc: 'https://rpc.space-pussy.cybernode.ai/',
    prefix: 'pussy',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-11',
    destinationChainId: 'space-pussy',
    destChannelId: 'channel-0',
    explorerUrlToTx: 'https://space-pussy.cyb.ai/network/bostrom/tx/{txHash}',
  },
  'juno-1': {
    chainId: 'juno-1',
    coinDecimals: 6,
    denom: 'JUNO',
    coinMinimalDenom: 'ujuno',
    rpc: 'https://rpc-juno.itastakers.com',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-10',
    destinationChainId: 'juno-1',
    destChannelId: 'channel-93',
    prefix: 'juno',
    explorerUrlToTx: 'https://www.mintscan.io/juno/txs/{txHash}',
  },
  'gravity-bridge-3': {
    chainId: 'gravity-bridge-3',
    coinDecimals: 6,
    denom: 'GRAV',
    coinMinimalDenom: 'ugraviton',
    rpc: 'https://gravitychain.io:26657',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-12',
    destinationChainId: 'gravity-bridge-3',
    destChannelId: 'channel-103',
    prefix: 'gravity',
    explorerUrlToTx: 'https://www.mintscan.io/gravity-bridge/txs/{txHash}',
  },
  'desmos-mainnet': {
    chainId: 'desmos-mainnet',
    coinDecimals: 6,
    denom: 'DSM',
    coinMinimalDenom: 'udsm',
    rpc: 'https://rpc.mainnet.desmos.network',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-13',
    destinationChainId: 'desmos-mainnet',
    destChannelId: 'channel-6',
    prefix: 'desmos',
    explorerUrlToTx: 'https://www.mintscan.io/desmos/txs/{txHash}',
  },
  'evmos_9001-2': {
    chainId: 'evmos_9001-2',
    rpc: 'https://rpc.evmos-9001-2.bronbro.io',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-9',
    destinationChainId: 'evmos_9001-2',
    destChannelId: 'channel-19',
    prefix: 'evmos',
    explorerUrlToTx: 'https://www.mintscan.io/evmos/txs/{txHash}',
    coinDecimals: 18,
    coinMinimalDenom: 'aevmos',
    denom: 'EVMOS',
  },
  'chihuahua-1': {
    chainId: 'chihuahua-1',
    coinDecimals: 6,
    denom: 'HUAHUA',
    coinMinimalDenom: 'uhuahua',
    rpc: 'https://rpc.chihuahua.wtf',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-13',
    destinationChainId: 'chihuahua-1',
    destChannelId: 'channel-6',
    prefix: 'chihuahua',
    explorerUrlToTx: 'https://www.mintscan.io/chihuahua/txs/{txHash}',
  },
  'axelar-dojo-1': {
    chainId: 'axelar-dojo-1',
    coinDecimals: 6,
    denom: 'AXL',
    coinMinimalDenom: 'uaxl',
    rpc: 'https://rpc-1.axelar.nodes.guru',
    sourceChainId: 'bostrom',
    sourceChannelId: 'channel-14',
    destinationChainId: 'axelar-dojo-1',
    destChannelId: 'channel-52',
    prefix: 'axelar',
    explorerUrlToTx: 'https://mintscan.io/axelar/txs/{txHash}',
  },
};

export default networkList;
