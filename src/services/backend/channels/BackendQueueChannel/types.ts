import { LinkDto } from 'src/services/CozoDb/types/dto';
import { IPFSContent } from 'src/services/ipfs/types';
import { SyncQueueItem } from '../../services/sync/services/ParticlesResolverQueue/types';

type DbQueueMessageType = 'link' | 'particle' | 'sync';
type DbSaveMessage<N extends DbQueueMessageType, T> = {
  type: N;
  data: T;
};
export type DbSaveLinkMessage = DbSaveMessage<'link', LinkDto[]>;
export type DbSaveParticleMessage = DbSaveMessage<'particle', IPFSContent>;
export type DbSyncQueueMessage = DbSaveMessage<'sync', SyncQueueItem[] | SyncQueueItem>;
export type QueueChannelMessage = DbSyncQueueMessage | DbSaveLinkMessage | DbSaveParticleMessage;
