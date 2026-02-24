import { Remote } from 'comlink';
import Unixfs from 'ipfs-unixfs';
import { DAGNode, util as DAGUtil } from 'ipld-dag-pb';
import { isString } from 'lodash';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { IpfsApi } from 'src/services/backend/workers/background/api/ipfsApi';
import { ParticleCid } from 'src/types/base';

export const isCID = (cid: string): boolean => {
  return cid.match(PATTERN_IPFS_HASH) !== null;
};

// eslint-disable-next-line import/prefer-default-export
export const getIpfsHash = (string: string): Promise<ParticleCid> =>
  new Promise((resolve, reject) => {
    const unixFsFile = new Unixfs('file', new TextEncoder().encode(string));

    const buffer = unixFsFile.marshal();
    DAGNode.create(buffer, (err, dagNode) => {
      if (err) {
        reject(new Error('Cannot create ipfs DAGNode'));
      }

      DAGUtil.cid(dagNode, (_error, cid) => {
        resolve(cid.toBaseEncodedString());
      });
    });
  });
export const addIfpsMessageOrCid = async (
  message: string | ParticleCid | File,
  { ipfsApi }: { ipfsApi: Remote<IpfsApi> | null }
) => {
  if (!ipfsApi) {
    throw Error('IpfsApi is not initialized');
  }

  return (
    isString(message) && message.match(PATTERN_IPFS_HASH)
      ? message
      : ((await ipfsApi!.addContent(message)) as string)
  ) as ParticleCid;
};
