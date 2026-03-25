/* eslint-disable import/no-unused-modules */

import { Option } from 'src/types';
import { concat as uint8ArrayConcat } from 'uint8arrays/concat';
import { CYBER_GATEWAY_URL, FILE_SIZE_DOWNLOAD } from '../config';
import {
  CallBackFuncStatus,
  IPFSContent,
  // getIpfsUserGatewanAndNodeType,
  IPFSContentMeta,
  IpfsContentSource,
  IpfsFileStats,
  IpfsNode,
} from '../types';
import cyberCluster from './cluster';
import { contentToUint8Array, createTextPreview, mimeToBaseContentType } from './content';
import ipfsCacheDb from './ipfsCacheDb';
import { getMimeFromUint8Array, toAsyncIterableWithMime } from './stream';

// Get data by CID from local storage
const loadIPFSContentFromDb = async (cid: string): Promise<Option<IPFSContent>> => {
  // TODO: enable, disabled for tests

  // TODO: use cursor
  const data = await ipfsCacheDb.get(cid);
  if (data?.length) {
    // TODO: use cursor
    const mime = await getMimeFromUint8Array(data);
    const contentType = mimeToBaseContentType(mime);

    const textPreview = createTextPreview(data, contentType);

    const meta: IPFSContentMeta = {
      type: 'file', // `TODO: ipfs refactor dir support ?
      size: data.length,
      sizeLocal: data.length,
      mime,
      contentType,
    };
    return { result: data, cid, meta, source: 'db', textPreview };
  }

  return undefined;
};

const emptyStats: IpfsFileStats = {
  type: 'file',
  size: undefined,
  sizeLocal: undefined,
  blocks: undefined,
};

const fetchIPFSContentStat = async (
  cid: string,
  node?: IpfsNode,
  signal?: AbortSignal
): Promise<IpfsFileStats> => {
  if (node) {
    const stats = await node.stat(cid, { signal });
    return stats;
  }
  return emptyStats;
};

const fetchIPFSContentFromNode = async (
  cid: string,
  node?: IpfsNode,
  controller?: AbortController
): Promise<Option<IPFSContent>> => {
  const controllerLegacy = controller || new AbortController();
  const { signal } = controllerLegacy;
  let timer: ReturnType<typeof setTimeout> | undefined;

  if (!node) {
    console.log('--------fetchIPFSContentFromNode NO NODE INTIALIZED--------');
    return undefined;
  }

  if (!controller) {
    timer = setTimeout(
      () => {
        controllerLegacy.abort();
      },
      1000 * 60 * 1
    );
  } // 1 min

  // TODO: cover ipns case
  try {
    // const stat = await node.files.stat(path, { signal });
    const startTime = Date.now();
    const stats = await fetchIPFSContentStat(cid, node, signal);
    const meta = stats as IPFSContentMeta;
    const statsDoneTime = Date.now();
    meta.statsTime = statsDoneTime - startTime;
    const allowedSize = stats.size ? stats.size < FILE_SIZE_DOWNLOAD : false;
    timer && clearTimeout(timer);

    switch (stats.type) {
      case 'directory': {
        // TODO: return directory structure
        return { cid, availableDownload: true, source: 'node', meta: stats };
      }
      default: {
        // Get sample of content
        const { value: firstChunk } = await node
          .cat(cid, { signal, length: 2048, offset: 0 })
          [Symbol.asyncIterator]()
          .next();

        meta.mime = await getMimeFromUint8Array(firstChunk);
        meta.contentType = mimeToBaseContentType(meta.mime);
        const fullyDownloaded = stats.size && stats.size > -1 && firstChunk.length >= stats.size;

        const textPreview = createTextPreview(firstChunk, meta.contentType);

        if (fullyDownloaded) {
          await ipfsCacheDb.add(cid, uint8ArrayConcat([firstChunk]));
        }

        // If all content fits in first chunk return byte-array instead iterable
        const stream = fullyDownloaded
          ? firstChunk
          : allowedSize
            ? node.cat(cid, { signal })
            : undefined;

        meta.catTime = Date.now() - statsDoneTime;

        // TODO: add to db flag that content is pinned TO local node
        // if already pinned skip pin
        if (!meta.local && allowedSize) {
          node.pin(cid);

          meta.pinTime = Date.now() - meta.catTime;
        } else {
          meta.pinTime = -1;
        }

        return {
          result: stream,
          textPreview,
          cid,
          meta,
          source: 'node',
        };
        // }
      }
    }
  } catch (error) {
    console.debug('error fetchIPFSContentFromNode', error);
    return undefined;
  }
};

const fetchIPFSContentFromGateway = async (
  cid: string,
  node?: IpfsNode,
  controller?: AbortController,
  headers?: Record<string, string>
): Promise<Option<IPFSContent>> => {
  // fetch META only from external node(toooo slow), TODO: fetch meta from cybernode
  const isExternalNode = node?.nodeType === 'external';

  const stats = isExternalNode
    ? await fetchIPFSContentStat(cid, node, controller?.signal)
    : emptyStats;

  const contentUrl = `${CYBER_GATEWAY_URL}/ipfs/${cid}`;
  const response = await fetch(contentUrl, {
    method: 'GET',
    signal: controller?.signal,
    headers,
  });
  if (response?.body) {
    // fetch doesn't provide any headers in our case :(

    // const contentLength = parseInt(
    //   response.headers['content-length'] || '-1',
    //   10
    // );
    // const contentType = response.headers['content-type'];

    // Extract meta if ipfs prob/node not started yet
    // if (!meta.mime) {
    //   meta = { ...meta, mime: contentType };
    // }

    // TODO: fix
    const flushResults = (chunks: Uint8Array[]) =>
      !isExternalNode ? ipfsCacheDb.add(cid, uint8ArrayConcat(chunks)) : Promise.resolve();

    const { mime, result, firstChunk } = await toAsyncIterableWithMime(response.body, flushResults);

    const contentType = mimeToBaseContentType(mime);

    const textPreview = createTextPreview(firstChunk, contentType);
    return {
      cid,
      textPreview,
      meta: { ...stats, mime, contentType },
      result,
      source: 'gateway',
      contentUrl,
    };
  }

  return undefined;
};

type fetchContentOptions = {
  controller?: AbortController;
  node?: IpfsNode;
  headers?: Record<string, string>;
};

async function fetchIpfsContent(
  cid: string,
  source: IpfsContentSource,
  options: fetchContentOptions
): Promise<Option<IPFSContent>> {
  const { node, controller, headers } = options;

  try {
    switch (source) {
      case 'db':
        return loadIPFSContentFromDb(cid);
      case 'node':
        return fetchIPFSContentFromNode(cid, node, controller);
      case 'gateway':
        return fetchIPFSContentFromGateway(cid, node, controller, headers);
      default:
        return undefined;
    }
  } catch (e) {
    console.log('----fetchIpfsContent error', e);
    return undefined;
  }
}

const getIPFSContent = async (
  cid: string,
  node?: IpfsNode,
  controller?: AbortController,
  callBackFuncStatus?: CallBackFuncStatus
): Promise<Option<IPFSContent>> => {
  const dataRsponseDb = await loadIPFSContentFromDb(cid);
  if (dataRsponseDb !== undefined) {
    return dataRsponseDb;
  }

  if (node) {
    callBackFuncStatus?.('trying to get with a node');
    const ipfsContent = await fetchIPFSContentFromNode(cid, node, controller);

    if (ipfsContent?.result) {
      return ipfsContent;
    }
    // Node failed or returned no content — fall through to gateway
  }

  callBackFuncStatus?.('trying to get with a gateway');
  const respnseGateway = await fetchIPFSContentFromGateway(cid, node, controller);

  return respnseGateway;
};

const catIPFSContentFromNode = (
  cid: string,
  node?: IpfsNode,
  offset?: number,
  controller?: AbortController
): AsyncIterable<Uint8Array> | undefined => {
  if (!node) {
    console.log('--------fetchIPFSContentFromNode NO NODE INTIALIZED TODO: cover case--------');
    return undefined;
  }

  // TODO: cover ipns case

  return node.cat(cid, { offset, signal: controller?.signal });
};

// const nodeContentFindProvs = async (
//   node: AppIPFS,
//   cid: string,
//   offset: number,
//   controller?: AbortController
// ): AsyncIterable<number> | undefined => {
//   if (!node) {
//     console.log(
//       '--------fetchIPFSContentFromNode NO NODE INTIALIZED TODO: cover case--------'
//     );
//     return undefined;
//   }

//   // TODO: cover ipns case
//   const path = `/ipfs/${cid}`;

//   const providers = node.dht.findProvs(path, {
//     signal: controller?.signal,
//   });

//   let count = 0;
//   for await (const provider of providers) {
//     //  console.log(provider.id.toString())
//     //  id: PeerId
//     // multiaddrs: Multiaddr[]
//     // protocols: string[]
//     count++;
//   }

//   return count;
// };

const addContenToIpfs = async (node: IpfsNode, content: File | string): Promise<Option<string>> => {
  let cid;
  if (node) {
    cid = await node.add(content);
  }
  // TODO: WARN - TMP solution make cluster call non-awaitable
  cyberCluster.add(content);
  // Save to local cache
  cid && (await ipfsCacheDb.add(cid, await contentToUint8Array(content)));
  return cid;
};

export { getIPFSContent, catIPFSContentFromNode, fetchIpfsContent, addContenToIpfs };
