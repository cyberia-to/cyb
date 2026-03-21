import { BehaviorSubject } from 'rxjs';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { initIpfsNode } from 'src/services/ipfs/node/factory';
import { CybIpfsNode, IpfsContentType, IpfsOptsType } from 'src/services/ipfs/types';
import QueueManager from 'src/services/QueueManager/QueueManager';
import { QueueItemCallback, QueueItemOptions } from 'src/services/QueueManager/types';
import { RuneEngine } from 'src/services/scripting/engine';

// eslint-disable-next-line import/prefer-default-export
export const createIpfsApi = (rune: RuneEngine, broadcastApi: BroadcastChannelSender) => {
  const ipfsInstance$ = new BehaviorSubject<CybIpfsNode | undefined>(undefined);
  const ipfsQueue = new QueueManager(ipfsInstance$, {
    rune,
  });
  const stopIpfs = async () => {
    const ipfsNode = ipfsInstance$.getValue();

    if (ipfsNode) {
      await ipfsNode.stop();
    }
    ipfsInstance$.next(undefined);
    broadcastApi.postServiceStatus('ipfs', 'inactive');
  };

  const startIpfs = async (ipfsOpts: IpfsOptsType) => {
    try {
      console.log('🔋 ipfs node init start');

      const ipfsNode = ipfsInstance$.getValue();
      if (ipfsNode) {
        console.log('🔋 Ipfs node already started!');
        setTimeout(() => broadcastApi.postServiceStatus('ipfs', 'started'), 0);
        return Promise.resolve();
      }

      console.time('🔋 ipfs initialized');
      broadcastApi.postServiceStatus('ipfs', 'starting');

      const newIpfsNode = await initIpfsNode(ipfsOpts);

      ipfsInstance$.next(newIpfsNode);
      setTimeout(() => broadcastApi.postServiceStatus('ipfs', 'started'), 0);
      return true;
    } catch (err) {
      console.log('🔋 ipfs node init error ', err);
      const msg = err instanceof Error ? err.message : String(err);
      broadcastApi.postServiceStatus('ipfs', 'error', msg);
      throw Error(msg);
    }
  };

  const api = {
    start: startIpfs,
    stop: stopIpfs,
    config: async () => ipfsInstance$.getValue()?.config,
    info: async () => ipfsInstance$.getValue()?.info(),
    fetchWithDetails: async (
      cid: string,
      parseAs?: IpfsContentType,
      controller?: AbortController
    ) => {
      const ipfsNode = ipfsInstance$.getValue();
      if (!ipfsNode) {
        throw new Error('ipfs node not initialized');
      }
      return ipfsNode.fetchWithDetails(cid, parseAs, controller);
    },
    enqueue: async (cid: string, callback: QueueItemCallback, options: QueueItemOptions) =>
      ipfsQueue.enqueue(cid, callback, options),
    enqueueAndWait: async (cid: string, options?: QueueItemOptions) =>
      ipfsQueue!.enqueueAndWait(cid, options),
    dequeue: async (cid: string) => ipfsQueue.cancel(cid),
    dequeueByParent: async (parent: string) => ipfsQueue.cancelByParent(parent),
    clearQueue: async () => ipfsQueue.clear(),
    addContent: async (content: string | File) => ipfsInstance$.getValue()?.addContent(content),
  };

  return { ipfsInstance$, ipfsQueue, api };
};

export type IpfsApi = ReturnType<typeof createIpfsApi>['api'];
