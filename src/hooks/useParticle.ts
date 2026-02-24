import { useEffect, useState } from 'react';
import { IPFSContentDetails, IPFSContentDetailsMutated } from 'src/services/ipfs/types';
import { parseArrayLikeToDetails } from 'src/services/ipfs/utils/content';
import { QueueItemStatus } from 'src/services/QueueManager/types';
import { Option } from 'src/types';
import { ParticleCid } from 'src/types/base';
import useQueueIpfsContent from './useQueueIpfsContent';
import useRuneMutation from './useRuneMutation';

const useParticle = (
  cid: ParticleCid,
  parentId: Option<string> = undefined,
  rank: Option<number> = undefined
) => {
  const { fetchParticle, status: queueItemStatus, content } = useQueueIpfsContent(parentId);
  const [details, setDetails] = useState<IPFSContentDetails | IPFSContentDetailsMutated>();
  const [status, setStatus] = useState<QueueItemStatus>('pending');
  const [hidden, setHidden] = useState(false);
  const [mutated, setMutated] = useState(false);
  const { state, mutatedDetails } = useRuneMutation(cid, details);
  useEffect(() => {
    if (cid && fetchParticle) {
      setStatus('pending');
      setDetails(undefined);
      setMutated(false);
      fetchParticle(cid, rank);
    }
  }, [cid, fetchParticle, rank]);

  useEffect(() => {
    if (queueItemStatus === 'completed') {
      parseArrayLikeToDetails(
        content,
        cid
        // (progress: number) => console.log(`${cid} progress: ${progress}`)
      ).then((details) => {
        setDetails(details);
        setStatus('completed');
      });
    } else {
      setStatus(queueItemStatus);
    }
  }, [content, queueItemStatus, cid]);

  useEffect(() => {
    if (state === 'done') {
      const hiddenByRune = !!mutatedDetails?.mutation && mutatedDetails.mutation === 'hidden';
      setHidden(hiddenByRune);
      setDetails(mutatedDetails);
      setMutated(true);
    }
  }, [state, mutatedDetails]);

  return { status, details, hidden, content, mutated };
};

export default useParticle;
