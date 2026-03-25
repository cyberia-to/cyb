import { CID_FOLLOW } from 'src/constants/app';

export const QUERY_GET_FOLLOWERS = {
  particle_from: { _eq: CID_FOLLOW },
};
