import { IpfsNodeType, IpfsNode, CybIpfsNode, IpfsOptsType } from '../types';
import KuboNode from './impl/kubo';
import {
  CYBERNODE_SWARM_ADDR_TCP,
  CYBER_NODE_SWARM_PEER_ID,
} from '../config';
import { withCybFeatures } from './mixins/withCybFeatures';

const nodeClassMap: Record<IpfsNodeType, new () => IpfsNode> = {
  external: KuboNode,
};

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export async function initIpfsNode(
  options: IpfsOptsType
): Promise<CybIpfsNode> {
  const { ipfsNodeType, ...restOptions } = options;

  const swarmPeerId = CYBER_NODE_SWARM_PEER_ID;

  const swarmPeerAddress = CYBERNODE_SWARM_ADDR_TCP;
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

  try {
    await instance.init({ url: restOptions.urlOpts });
  } catch (error) {
    console.log('[Worker] initIpfsNode instance init failed', error);
  }
  console.log('[Worker] initIpfsNode after instance init');

  await instance.reconnectToSwarm();
  return instance;
}
