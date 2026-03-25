import { PinType } from 'ipfs-core-types/src/pin';
import { Transaction } from 'src/services/backend/services/indexer/types';
import {
  SenseChatExtension,
  SenseLinkMeta,
  SenseTransactionMeta,
} from 'src/services/backend/types/sense';
import { IpfsContentType } from 'src/services/ipfs/types';
import { QueuePriority } from 'src/services/QueueManager/types';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { DtoToEntity } from 'src/types/dto';

type PinEntryType = Exclude<PinType, 'all'>;
// example of db optimization for classifiers

export const PinTypeMap: Record<PinEntryType, number> = {
  indirect: -1,
  direct: 0,
  recursive: 1,
};

export enum EntryType {
  transactions = 1,
  particle = 2,
  chat = 3,
}

// Transaction if formed by frontend
// Should be replaced after sync

export type PinDbEntity = {
  cid: string;
  type: keyof typeof PinTypeMap;
};

export type TransactionDbEntity = {
  hash: string;
  index: number;
  type: string;
  timestamp: number;
  block_height: number;
  value: Transaction['value'];
  success: boolean;
  memo: string;
  neuron: NeuronAddress;
};

type SyncItemMeta = DtoToEntity<(SenseLinkMeta | SenseTransactionMeta) & SenseChatExtension>;

export type SyncStatusDbEntity = {
  entry_type: EntryType;
  id: NeuronAddress | ParticleCid;
  owner_id: NeuronAddress;
  timestamp_update: number;
  timestamp_read: number;
  disabled: boolean;
  unread_count: number;
  meta: SyncItemMeta;
};

export type ParticleDbEntity = {
  id: ParticleCid;
  size: number;
  size_local: number;
  blocks: number;
  mime: string;
  type: IpfsContentType;
  text: string;
};

export type LinkDbEntity = {
  from: ParticleCid;
  to: ParticleCid;
  neuron: NeuronAddress;
  timestamp: number;
  transaction_hash: string;
};

export type ConfigDbEntity = {
  key: string;
  group_key: string;
  value: NonNullable<unknown>;
};

export enum SyncQueueStatus {
  pending = 0,
  executing = 1,
  done = 2,
  error = -1,
}

export enum SyncQueueJobType {
  particle = 0,
  embedding = 1,
}

export type SyncQueueKey = {
  id: string;
  job_type: SyncQueueJobType;
};

export type SyncQueueDbEntity = SyncQueueKey & {
  data: string;
  status: SyncQueueStatus;
  priority: QueuePriority | number;
};

export type CommunityDbEntity = {
  ownerId: NeuronAddress;
  particle: ParticleCid;
  neuron: NeuronAddress;
  name: string;
  following: boolean;
  follower: boolean;
};

export type EmbeddinsDbEntity = {
  cid: ParticleCid;
  vec: number[];
};

export type DbEntity =
  | TransactionDbEntity
  | ParticleDbEntity
  | SyncStatusDbEntity
  | ConfigDbEntity
  | SyncQueueDbEntity
  | EmbeddinsDbEntity;
