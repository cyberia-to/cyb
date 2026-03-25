import { CID_FOLLOW, CID_TWEET } from 'src/constants/app';
import { SyncEntryName } from 'src/services/backend/types/services';

export const MY_PARTICLES_SYNC_INTERVAL = 5 * 60 * 1000; // 60 sec
export const MY_FRIENDS_SYNC_INTERVAL = 5 * 60 * 1000; // 60 sec
export const IPFS_SYNC_INTERVAL = 15 * 60 * 1000; // 15 minutes

export const MAX_DATABASE_PUT_SIZE = 500;

export const MAX_LINKS_RESOLVE_BATCH = 20;

export const DAY_IN_MS = 24 * 60 * 60 * 1000;

export const SENSE_FRIEND_PARTICLES = [CID_TWEET, CID_FOLLOW];

export const SYNC_ENTRIES_TO_TRACK_PROGRESS = [
  'my-friends',
  'particles',
  'transactions',
] as SyncEntryName[];
