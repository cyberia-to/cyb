import { Cluster } from '@nftstorage/ipfs-cluster';
import { AddResponse, PinResponse } from '@nftstorage/ipfs-cluster/dist/src/interface';
import { IPFS_CLUSTER_URL } from '../config';

const cyberCluster = () => {
  const cluster = new Cluster(IPFS_CLUSTER_URL);

  const add = async (file: File | string): Promise<AddResponse | PinResponse | undefined> => {
    const dataFile = typeof file === 'string' ? new File([file], 'file.txt') : file;
    return cluster.add(dataFile, { cidVersion: 0, rawLeaves: false });
  };

  const status = async (cid: string) => cluster.status(cid);
  return { add, status };
};

export default cyberCluster();
