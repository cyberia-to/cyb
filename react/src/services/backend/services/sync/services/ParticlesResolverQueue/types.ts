import { SyncQueueDto } from 'src/services/CozoDb/types/dto';

export type SyncQueueItem = Omit<SyncQueueDto, 'status'> & {
  status?: SyncQueueDto['status'];
};
