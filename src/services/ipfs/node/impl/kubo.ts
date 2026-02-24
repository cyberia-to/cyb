import { multiaddr } from '@multiformats/multiaddr';
import { create as createKuboClient, IPFSHTTPClient } from 'kubo-rpc-client';
import { CYBER_GATEWAY_URL } from '../../config';
import {
  AbortOptions,
  CatOptions,
  InitOptions,
  IpfsFileStats,
  IpfsNode,
  IpfsNodePrperties,
  IpfsNodeType,
} from '../../types';
import { stringToCid, stringToIpfsPath } from '../../utils/cid';

class KuboNode implements IpfsNode {
  readonly nodeType: IpfsNodeType = 'external';

  private node?: IPFSHTTPClient;

  private _config: IpfsNodePrperties = {};

  get config() {
    return this._config;
  }

  private _isStarted = false;

  get isStarted() {
    return this._isStarted;
  }

  private async initConfig() {
    const response = await this.node!.config.get('Addresses.Gateway');
    if (!response) {
      return { gatewayUrl: CYBER_GATEWAY_URL };
    }
    const address = multiaddr(response as string).nodeAddress();

    return { gatewayUrl: `http://${address.address}:${address.port}` };
  }

  async init(options?: InitOptions) {
    console.log('🔋 ipfs KuboNode init', options);

    this.node = createKuboClient(options);
    console.log('🔋 ipfs KuboNode initialized', this.node);

    console.log('🔋 ipfs config init');
    this._config = await this.initConfig();
    console.log('🔋 ipfs config initialized', this._config);

    if (typeof window !== 'undefined') {
      window.node = this.node;
      window.toCid = stringToCid;
    }

    // const kuboAddresses = (await this.node.swarm.localAddrs()).map((a) =>
    //   a.toString()
    // );
    // console.log('🔋 ipfs - Kubo addrs', kuboAddresses);

    this._isStarted = true;
  }

  async stat(cid: string, options: AbortOptions = {}): Promise<IpfsFileStats> {
    return this.node!.files.stat(stringToIpfsPath(cid), {
      ...options,
      withLocal: true,
      size: true,
    }).then((result) => {
      const { type, size, sizeLocal, local, blocks } = result;
      return {
        type,
        size: size || -1,
        sizeLocal: sizeLocal || -1,
        blocks,
      };
    });
  }

  cat(cid: string, options: CatOptions = {}) {
    return this.node!.cat(stringToCid(cid), options);
  }

  async add(content: File | string, options: AbortOptions = {}) {
    return (await this.node!.add(content, options)).cid.toString();
  }

  async pin(cid: string, options: AbortOptions = {}) {
    return (await this.node!.pin.add(stringToCid(cid), options)).toString();
  }

  async getPeers() {
    return (await this.node!.swarm.peers()).map((c) => c.peer.toString());
  }

  async stop() { }

  async start() { }

  async connectPeer(address: string) {
    const addr = multiaddr(address);
    await this.node!.bootstrap.add(addr);

    await this.node!.swarm.connect(addr);
    return true;
  }

  ls() {
    return this.node!.pin.ls();
  }

  async info() {
    const { repoSize } = await this.node!.stats.repo();

    const responseId = await this.node!.id();
    const { agentVersion, id } = responseId;
    return { id: id.toString(), agentVersion, repoSize };
  }
}

// eslint-disable-next-line import/no-unused-modules
export default KuboNode;
