import { IPFSNodes, IpfsOptsType } from './types';
import safeLocalStorage from 'src/utils/safeLocalStorage';

export const CYBER_NODE_SWARM_PEER_ID =
  'QmUgmRxoLtGERot7Y6G7UyF6fwvnusQZfGR15PuE6pY3aB';

export const CYBERNODE_SWARM_ADDR_WSS = `/dns4/swarm.io.cybernode.ai/tcp/443/wss/p2p/${CYBER_NODE_SWARM_PEER_ID}`;
export const CYBERNODE_SWARM_ADDR_TCP = `/ip4/88.99.105.146/tcp/4001/p2p/${CYBER_NODE_SWARM_PEER_ID}`;

export const IPFS_CLUSTER_URL = 'https://io.cybernode.ai';

export const CYBER_GATEWAY_URL = 'https://gateway.ipfs.cybernode.ai';

export const FILE_SIZE_DOWNLOAD = 20 * 10 ** 6;

// Desktop Tauri runs a local Kubo daemon; mobile Tauri and browser use cybernode
const isTauriMobile =
  !!process.env.IS_TAURI &&
  typeof navigator !== 'undefined' &&
  /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

const isTauriDesktop = !!process.env.IS_TAURI && !isTauriMobile;

const defaultIpfsOpts: IpfsOptsType = {
  ipfsNodeType: IPFSNodes.EXTERNAL,
  urlOpts: isTauriDesktop
    ? '/ip4/127.0.0.1/tcp/5001'
    : 'https://io.cybernode.ai',
  userGateway: isTauriDesktop
    ? 'http://127.0.0.1:8080'
    : 'https://gateway.ipfs.cybernode.ai',
};

export const getIpfsOpts = (): IpfsOptsType => {
  const stored = safeLocalStorage.getJSON<Partial<IpfsOptsType>>('ipfsState', {});
  const ipfsOpts = { ...defaultIpfsOpts, ...stored };

  // Always use EXTERNAL — Tauri uses local Kubo, browser uses cybernode
  ipfsOpts.ipfsNodeType = IPFSNodes.EXTERNAL;

  safeLocalStorage.setJSON('ipfsState', ipfsOpts);

  return ipfsOpts;
};
