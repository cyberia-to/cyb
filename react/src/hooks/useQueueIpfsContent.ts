import { proxy } from 'comlink';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { FetchWithDetailsFunc, IPFSContent, IpfsContentSource } from 'src/services/ipfs/types';
import {
  FetchParticleAsync,
  QueueItemOptions,
  QueueItemStatus,
  QueuePriority,
} from 'src/services/QueueManager/types';
import { Option } from 'src/types';

type UseIpfsContentReturn = {
  isReady: boolean;
  status: QueueItemStatus;
  source?: IpfsContentSource;
  content: Option<IPFSContent>;
  clear?: () => Promise<void>;
  cancel?: (cid: string) => Promise<void>;
  fetchParticle?: (cid: string, rank?: number) => Promise<void>;
  fetchParticleAsync?: FetchParticleAsync;
  fetchWithDetails?: FetchWithDetailsFunc;
};

function useQueueIpfsContent(parentId?: string): UseIpfsContentReturn {
  const [status, setStatus] = useState<QueueItemStatus>();
  const [source, setSource] = useState<IpfsContentSource | undefined>();
  const [content, setContent] = useState<Option<IPFSContent>>();
  const prevParentIdRef = useRef<string | undefined>();

  const {
    // backgroundWorker: backendApi,
    ipfsApi,
    isIpfsInitialized,
  } = useBackend();

  const fetchParticle = useCallback(
    async (cid: string, _rank?: number) => {
      setContent(undefined);
      setStatus('pending');
      setSource(undefined);

      const callback = (
        _cid: string,
        status: QueueItemStatus,
        source: IpfsContentSource,
        result: Option<IPFSContent>
      ) => {
        setStatus(status);
        setSource(source);
        if (status === 'completed') {
          (async () => Promise.resolve(result).then(setContent))();
        }
      };
      await ipfsApi?.enqueue(cid, proxy(callback), {
        parent: parentId,
        priority: QueuePriority.URGENT, //rank || 0,
        viewPortPriority: 0,
      });
    },
    [parentId, ipfsApi]
  );

  const fetchParticleAsync = useCallback(
    async (cid: string, options?: QueueItemOptions) => ipfsApi?.enqueueAndWait(cid, options),
    [ipfsApi]
  );

  useEffect(() => {
    if (prevParentIdRef.current !== parentId) {
      if (prevParentIdRef.current) {
        ipfsApi?.dequeueByParent(prevParentIdRef.current);
      }
      prevParentIdRef.current = parentId;
    }
  }, [parentId, ipfsApi]);

  return {
    isReady: !!ipfsApi && isIpfsInitialized,
    status,
    source,
    content,
    cancel: ipfsApi ? (cid: string) => ipfsApi.dequeue(cid) : undefined,
    clear: ipfsApi ? async () => ipfsApi.clearQueue() : undefined,
    fetchParticle: ipfsApi ? fetchParticle : undefined,
    fetchParticleAsync: ipfsApi ? fetchParticleAsync : undefined,
    fetchWithDetails: ipfsApi ? ipfsApi.fetchWithDetails : undefined,
  };
}

export default useQueueIpfsContent;
