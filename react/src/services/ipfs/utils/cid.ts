import { CID } from 'multiformats/cid';

export const stringToCid = (s: string) => CID.parse(s);
export const stringToIpfsPath = (s: string) => `/ipfs/${s}`;
