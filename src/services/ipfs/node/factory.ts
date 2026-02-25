import {
  CYBER_NODE_SWARM_PEER_ID,
  CYBERNODE_SWARM_ADDR_TCP,
  CYBERNODE_SWARM_ADDR_WSS,
} from '../config';
import { CybIpfsNode, IpfsNode, IpfsNodeType, IpfsOptsType } from '../types';
import HeliaNode from './impl/helia';
import JsIpfsNode from './impl/js-ipfs';
import KuboNode from './impl/kubo';
import { withCybFeatures } from './mixins/withCybFeatures';

const nodeClassMap: Record<IpfsNodeType, new () => IpfsNode> = {
  external: KuboNode,
};

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export async function initIpfsNode(options: IpfsOptsType): Promise<CybIpfsNode> {
  const { ipfsNodeType, ...restOptions } = options;

  const swarmPeerId = CYBER_NODE_SWARM_PEER_ID;

  const swarmPeerAddress =
    ipfsNodeType === 'external' ? CYBERNODE_SWARM_ADDR_TCP : CYBERNODE_SWARM_ADDR_WSS;
  console.log('[Worker] initIpfsNode', {
    swarmPeerId,
    swarmPeerAddress,
    ipfsNodeType,
  });

  const EnhancedClass = withCybFeatures(nodeClassMap[ipfsNodeType], {
    swarmPeerId,
    swarmPeerAddress,
  });
  console.log('[Worker] initIpfsNode', { EnhancedClass });

  const instance = new EnhancedClass();
  console.log('[Worker] initIpfsNode before init', { instance });

  await instance.init({ url: restOptions.urlOpts, userGateway: restOptions.userGateway });
  console.log('[Worker] initIpfsNode after instance init');

  // Swarm connection is best-effort — don't block initialization
  await instance.reconnectToSwarm().catch((err) => {
    console.warn('[Worker] reconnectToSwarm failed (non-fatal):', err?.message);
  });
  return instance;
}
