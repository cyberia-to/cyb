import { noise } from '@chainsafe/libp2p-noise';
import { yamux } from '@chainsafe/libp2p-yamux';
import { IDBBlockstore } from 'blockstore-idb';
import { IDBDatastore } from 'datastore-idb';
import { createHelia, Helia, Pin } from 'helia';
import { createLibp2p } from 'libp2p';

// import { mplex } from '@libp2p/mplex';

import { AddOptions, UnixFS, unixfs } from '@helia/unixfs';
import { bootstrap } from '@libp2p/bootstrap';
import { webRTC } from '@libp2p/webrtc';
import { webSockets } from '@libp2p/websockets';
// webTransport removed — package not installed, WebSocket+WebRTC sufficient
import { multiaddr } from '@multiformats/multiaddr';
import { LsResult } from 'ipfs-core-types/src/pin';
import { circuitRelayTransport } from 'libp2p/circuit-relay';
import { identifyService } from 'libp2p/identify';
import { CYBER_GATEWAY_URL } from '../../config';
import { AbortOptions, CatOptions, IpfsFileStats, IpfsNode, IpfsNodeType } from '../../types';
// import { all } from '@libp2p/websockets/filters';
import { stringToCid } from '../../utils/cid';

async function* mapToLsResult(iterable: AsyncIterable<Pin>): AsyncIterable<LsResult> {
  // eslint-disable-next-line no-restricted-syntax
  for await (const item of iterable) {
    const { cid, metadata } = item;
    yield { cid: cid.toV0(), metadata, type: 'recursive' };
  }
}

const libp2pFactory = async (datastore: IDBDatastore, bootstrapList: string[] = []) => {
  const libp2p = await createLibp2p({
    datastore,
    transports: [
      webSockets(),
      webRTC({
        rtcConfiguration: {
          iceServers: [
            {
              urls: ['stun:stun.l.google.com:19302'],
            },
          ],
        },
      }),
      circuitRelayTransport({
        discoverRelays: 1,
      }),
    ],
    connectionEncryption: [noise()],
    streamMuxers: [yamux()],
    connectionManager: {
      maxConnections: 10,
      minConnections: 2,
      autoDialInterval: 30000, // slower autodial (default 10s)
    },
    connectionGater: {
      denyDialMultiaddr: () => {
        return false;
      },
    },
    peerDiscovery: [
      bootstrap({
        list: bootstrapList,
      }),
    ],
    services: {
      identify: identifyService(),
    },
  });
  return libp2p;
};

const addOptionsV0: Partial<AddOptions> = {
  cidVersion: 0,
  rawLeaves: false,
};

class HeliaNode implements IpfsNode {
  readonly nodeType: IpfsNodeType = 'helia';

  get config() {
    return { gatewayUrl: CYBER_GATEWAY_URL };
  }

  private _isStarted = false;

  get isStarted() {
    return this._isStarted;
  }

  private node?: Helia;

  private fs?: UnixFS;

  async init() {
    const blockstore = new IDBBlockstore('helia-bs');
    await blockstore.open();

    const datastore = new IDBDatastore('helia-ds');
    await datastore.open();

    const bootstrapList = [
      '/dns4/swarm.io.cybernode.ai/tcp/443/wss/p2p/QmUgmRxoLtGERot7Y6G7UyF6fwvnusQZfGR15PuE6pY3aB',
      '/dns4/helia.cybernode.ai/tcp/4444/wss/p2p/12D3KooWLKrsc4qYf3Z9EeUviKBPrp493AEHkrs38wXBs6NuKo2C',
    ];
    const libp2p = await libp2pFactory(datastore, bootstrapList);

    this.node = await createHelia({ blockstore, datastore, libp2p });

    this.fs = unixfs(this.node);

    if (typeof window !== 'undefined') {
      window.libp2p = libp2p;
      window.node = this.node;
      window.fs = this.fs;
      window.toCid = stringToCid;
    }

    console.log('IPFS - Helia started, peers:', bootstrapList.length);

    this._isStarted = true;
  }

  async stat(cid: string, options: AbortOptions = {}): Promise<IpfsFileStats> {
    return this.fs!.stat(stringToCid(cid), options).then((result) => {
      const { type, fileSize, localFileSize, blocks, dagSize, mtime } = result;
      return {
        type,
        size: fileSize || -1,
        sizeLocal: localFileSize || -1,
        blocks,
      };
    });
  }

  cat(cid: string, options: CatOptions = {}) {
    return this.fs!.cat(stringToCid(cid), options);
  }

  async add(content: File | string, options: AbortOptions = {}) {
    // Options to keep CID in V0 format 'Qm....';
    const optionsV0 = {
      ...options,
      ...addOptionsV0,
    } as Partial<AddOptions>;

    let cid;

    if (content instanceof File) {
      const fileName = content.name;
      const arrayBuffer = await content.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);
      cid = await this.fs!.addFile({ path: fileName, content: data }, optionsV0);
    } else {
      const data = new TextEncoder().encode(content);
      cid = await this.fs!.addBytes(data, optionsV0);
    }
    // console.log('----added to helia', cid.toString());
    this.pin(cid.toString(), options);
    return cid.toString();
  }

  async pin(cid: string, options: AbortOptions = {}) {
    const cid_ = stringToCid(cid);
    const isPinned = await this.node?.pins.isPinned(cid_, options);
    if (!isPinned) {
      const _pinResult = (await this.node?.pins.add(cid_, options))?.cid.toString();
      // console.log('------pin', pinResult);
    }
    // console.log('------pinned', cid, isPinned);
    return undefined;
  }

  async getPeers() {
    return this.node!.libp2p!.getConnections().map((c) => c.remotePeer.toString());
  }

  async stop() {
    await this.node?.stop();
  }

  async start() {
    await this.node?.start();
  }

  async connectPeer(address: string) {
    const _conn = await this.node!.libp2p!.dial(multiaddr(address));
    return true;
  }

  ls() {
    const result = mapToLsResult(this.node!.pins.ls());
    return result;
  }

  async info() {
    const id = this.node!.libp2p.peerId.toString();
    const agentVersion = this.node!.libp2p!.services!.identify!.host!.agentVersion as string;
    return { id, agentVersion, repoSize: -1 };
  }
}

// eslint-disable-next-line import/no-unused-modules
export default HeliaNode;
