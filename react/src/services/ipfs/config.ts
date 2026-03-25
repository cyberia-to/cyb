import safeLocalStorage from 'src/utils/safeLocalStorage';
import { IPFSNodes, IpfsOptsType } from './types';

export const CYBER_NODE_SWARM_PEER_ID = 'QmUgmRxoLtGERot7Y6G7UyF6fwvnusQZfGR15PuE6pY3aB';

export const CYBERNODE_SWARM_ADDR_WSS = `/dns4/swarm.io.cybernode.ai/tcp/443/wss/p2p/${CYBER_NODE_SWARM_PEER_ID}`;
export const CYBERNODE_SWARM_ADDR_TCP = `/ip4/88.99.105.146/tcp/4001/p2p/${CYBER_NODE_SWARM_PEER_ID}`;

export const IPFS_CLUSTER_URL = 'https://io.cybernode.ai';

export const CYBER_GATEWAY_URL = 'https://gateway.ipfs.cybernode.ai';

export const FILE_SIZE_DOWNLOAD = 20 * 10 ** 6;

const defaultIpfsOpts: IpfsOptsType = {
  ipfsNodeType: IPFSNodes.EXTERNAL,
  urlOpts: 'https://io.cybernode.ai',
  userGateway: 'https://gateway.ipfs.cybernode.ai',
};

function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

export const getIpfsOpts = (): IpfsOptsType => {
  const stored = safeLocalStorage.getJSON<Partial<IpfsOptsType>>('ipfsState', {});
  const ipfsOpts = { ...defaultIpfsOpts, ...stored };

  // Validate node type
  if (!Object.values(IPFSNodes).includes(ipfsOpts.ipfsNodeType as IPFSNodes)) {
    ipfsOpts.ipfsNodeType = IPFSNodes.EXTERNAL;
  }

  // Discard invalid URLs
  if (ipfsOpts.urlOpts && !isValidUrl(ipfsOpts.urlOpts)) {
    ipfsOpts.urlOpts = defaultIpfsOpts.urlOpts;
    ipfsOpts.userGateway = defaultIpfsOpts.userGateway;
  }

  safeLocalStorage.setJSON('ipfsState', ipfsOpts);

  return ipfsOpts;
};
