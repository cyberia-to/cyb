import { multiaddr } from '@multiformats/multiaddr';
import { create as createJsIpfsClient, IPFS } from 'ipfs-core';
import { CYBER_GATEWAY_URL } from '../../config';
import { AbortOptions, CatOptions, IpfsFileStats, IpfsNode, IpfsNodeType } from '../../types';
import { stringToCid, stringToIpfsPath } from '../../utils/cid';
import configIpfs from './configs/jsIpfsConfig';

class JsIpfsNode implements IpfsNode {
  readonly nodeType: IpfsNodeType = 'embedded';

  get config() {
    return { gatewayUrl: CYBER_GATEWAY_URL };
  }

  private _isStarted = false;

  get isStarted() {
    return this._isStarted;
  }

  private node?: IPFS;

  async init() {
    this.node = await createJsIpfsClient(configIpfs());
    if (typeof window !== 'undefined') {
      window.node = this.node;
      window.toCid = stringToCid;
    }

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

  async stop() {}
  async start() {}

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
    const response = await this.node!.stats.repo();
    const repoSize = Number(response.repoSize);

    const responseId = await this.node!.id();
    const { agentVersion, id } = responseId;
    return { id: id.toString(), agentVersion, repoSize };
  }
}

// eslint-disable-next-line import/no-unused-modules
export default JsIpfsNode;
