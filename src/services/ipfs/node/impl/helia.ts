import { noise } from '@chainsafe/libp2p-noise';
import { yamux } from '@chainsafe/libp2p-yamux';
import { IDBBlockstore } from 'blockstore-idb';
import { IDBDatastore } from 'datastore-idb';
import { createHelia, Helia, Pin } from 'helia';
import { createLibp2p } from 'libp2p';

// import { mplex } from '@libp2p/mplex';

import { AddOptions, UnixFS, unixfs } from '@helia/unixfs';
import { bootstrap } from '@libp2p/bootstrap';
import { webRTC, webRTCDirect } from '@libp2p/webrtc';
import { webSockets } from '@libp2p/websockets';
// webTransport removed — package not installed, WebSocket+WebRTC sufficient
import { multiaddr, protocols } from '@multiformats/multiaddr';
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
    // addresses: {
    //   listen: [
    //     '/ip4/127.0.0.1/tcp/0',
    //     '/dns4/swarm.io.cybernode.ai/tcp/443/wss/p2p/QmUgmRxoLtGERot7Y6G7UyF6fwvnusQZfGR15PuE6pY3aB',
    //   ],
    // },
    transports: [
      webSockets(),
      webRTC({
        rtcConfiguration: {
          iceServers: [
            {
              urls: [
                'stun:stun.l.google.com:19302',
                'stun:global.stun.twilio.com:3478',
                'STUN:freestun.net:3479',
                'STUN:stun.bernardoprovenzano.net:3478',
                'STUN:stun.aa.net.uk:3478',
              ],
            },
            {
              credential: 'free',
              username: 'free',
              urls: ['TURN:freestun.net:3479', 'TURNS:freestun.net:5350'],
            },
          ],
        },
      }),
      webRTCDirect(),
      circuitRelayTransport({
        discoverRelays: 1,
      }),
    ],
    connectionEncryption: [noise()],
    streamMuxers: [yamux()],
    connectionGater: {
      denyDialMultiaddr: () => {
        return false;
        // by default we refuse to dial local addresses from the browser since they
        // are usually sent by remote peers broadcasting undialable multiaddrs but
        // here we are explicitly connecting to a local node so do not deny dialing
        // any discovered address
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
      '/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN',
      '/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa',
      '/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb',
      '/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt',
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

    // DEBUG
    libp2p.addEventListener('peer:connect', (evt) => {
      const peerId = evt.detail.toString();
      const conn = libp2p.getConnections(peerId) || [];
      const transportsByAddr = Object.fromEntries(
        conn.map((c) => [
          c.remoteAddr.toString(),
          c.remoteAddr.protoCodes().map((v) => protocols(v)?.name),
        ])
      );
      console.debug(`Connected to ${peerId}`, transportsByAddr);

      // console.log(
      //   '---------ppppp',
      //   peerId,
      //   conn,
      //   conn?.remoteAddr.protoCodes().map((v) => protocols(v)?.name)
      // ); //.includes(WEBRTC_CODE)
      // if (conn && conn.stat) {
      //   const transport = conn.stat.transport; // This might vary based on libp2p version
      //   console.log(`Connected to ${peerId} using transport ${transport}`);
      // } else {
      //   console.log(`Connected to ${peerId}`);
      // }
    });
    libp2p.addEventListener('peer:disconnect', (evt) => {
      console.debug(`Disconnected from ${evt.detail.toString()}`);
    });
    console.log(
      'IPFS - Helia addrs',
      libp2p.getMultiaddrs().map((a) => a.toString())
    );
    // const webrtcConn = await libp2p.dial(
    //   multiaddr(
    //     '/ip4/127.0.0.1/udp/4001/quic-v1/webtransport/certhash/uEiDHumbyZRFV1Av7qH9-2l5HGgU2a2UqM6eloqO0vYz5pQ/certhash/uEiDD_TuVgih5_ua31Z4MVbNq7WSw095UAQmZqdUFMDTVRA/p2p/12D3KooWEYGfgK4dEY3spfuDKVq6Jpiyj4KxP1r6HS5RFp5WHebz'
    //   )
    // );
    // console.log('----webrtcConn', webrtcConn);

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
