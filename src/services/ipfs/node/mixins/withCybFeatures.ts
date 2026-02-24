import { CybIpfsNode, IpfsContentType, IpfsNode } from '../../types';
import { parseArrayLikeToDetails } from '../../utils/content';
import { addContenToIpfs, getIPFSContent } from '../../utils/utils-ipfs';

type WithCybFeaturesOptions = {
  swarmPeerId: string;
  swarmPeerAddress: string;
};

function withCybFeatures<TBase extends new (...args: any[]) => IpfsNode>(
  Base: TBase,
  options: WithCybFeaturesOptions
) {
  return class CybIpfsNodeMixin extends Base implements CybIpfsNode {
    async fetchWithDetails(
      cid: string,
      parseAs?: IpfsContentType,
      abortController?: AbortController
    ) {
      const content = await getIPFSContent(cid, this, abortController);

      const details = await parseArrayLikeToDetails(content, cid);
      return !parseAs ? details : details?.type === parseAs ? details : undefined;
    }

    async addContent(content: File | string) {
      return addContenToIpfs(this, content);
    }

    async isConnectedToSwarm() {
      const peers = await super.getPeers();
      return !!peers.find((peerId) => peerId === options.swarmPeerId);
    }

    async reconnectToSwarm(forced = false) {
      const isConnectedToSwarm = await this.isConnectedToSwarm();
      if (!isConnectedToSwarm || forced) {
        // TODO: refactor using timeout for node config

        //   const needToReconnect =
        //     Date.now() - lastConnectedTimestamp <
        //     DEFAULT_CONNECTION_LIFETIME_SECONDS;
        super
          .connectPeer(options.swarmPeerAddress)
          .then(() => {
            console.log(`🐝 connected to swarm - ${options.swarmPeerAddress}`);
            return true;
          })
          .catch((err) => {
            console.log(`Can't connect to swarm ${options.swarmPeerAddress}: ${err.message}`);
            return false;
          });
      }
    }
  };
}

export { withCybFeatures };
