import DOMPurify from 'dompurify';
import { IpfsApi } from 'src/services/backend/workers/background/api/ipfsApi';
import { IPFSContentDetails, IPFSContentDetailsMutated } from 'src/services/ipfs/types';
import { parseArrayLikeToDetails } from 'src/services/ipfs/utils/content';
import { QueuePriority } from 'src/services/QueueManager/types';
import { ParticleCid } from 'src/types/base';
import { RuneEngine } from '../engine';

/**
 * Execute 'particle' script to post process item: modify cid or content, or hide from view
 * @param item
 * @param details
 * @param ipfsQueue
 * @returns
 */
// eslint-disable-next-line import/prefer-default-export, import/no-unused-modules
export async function postProcessIpfContent(
  cid: ParticleCid,
  details: IPFSContentDetails,
  rune: RuneEngine,
  ipfsApi: IpfsApi
): Promise<IPFSContentDetailsMutated> {
  try {
    const { type: contentType, content } = details!;

    const mutation = await rune.personalProcessor({
      cid,
      contentType,
      content,
    });

    if (mutation.action === 'cid_result' && mutation.cid) {
      // refectch content from new cid
      const result = await ipfsApi.enqueueAndWait(mutation.cid, {
        priority: QueuePriority.URGENT,
      });
      const mutatedDetails = await parseArrayLikeToDetails(result.result, mutation.cid);
      // console.log('----cid_result', cid, details, mutation, result);

      if (result) {
        return {
          ...mutatedDetails,
          cidBefore: cid,
          mutation: 'modified',
        };
      }
    }

    if (mutation.action === 'content_result') {
      const sanitized =
        typeof mutation.content === 'string'
          ? DOMPurify.sanitize(mutation.content)
          : mutation.content;
      return {
        ...details,
        content: sanitized,
        text: sanitized,
        mutation: 'modified',
      };
    }

    if (mutation.action === 'hide') {
      return { ...details, mutation: 'hidden' };
    }

    if (mutation.action === 'error') {
      return { ...details, mutation: 'error' };
    }

    return details as IPFSContentDetailsMutated;
  } catch (e) {
    console.log('----exc', e);
    return { ...details, mutation: 'error' };
  }
}
