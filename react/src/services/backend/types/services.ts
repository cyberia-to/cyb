import { setDefaultAccount } from 'src/redux/features/pocket';
import { SyncCommunityResult } from 'src/services/community/community';
import { IndexedDbWriteMessage } from '../../CozoDb/types/types';

export type SyncEntryName = 'pin' | 'particles' | 'transactions' | 'resolver' | 'my-friends';

export type ProgressTracking = {
  totalCount: number;
  completeCount: number;
  estimatedTime: number;
};

export type SyncProgress = Partial<{
  status: 'active' | 'initialized' | 'listen' | 'estimating' | 'in-progress' | 'error' | 'inactive';

  progress: ProgressTracking;
  done: boolean;
  error: string;
  message: string;
}>;

export type ServiceStatus = 'inactive' | 'starting' | 'started' | 'error';

export type SyncEntryStatus = Record<SyncEntryName, SyncProgress>;
export type SyncMlEntryStatus = Record<string, SyncProgress>;

export type SyncState = {
  entryStatus: Partial<SyncEntryStatus>;
  lastError?: string;
  totalEstimatedTime: number;
  message: string;
  inProgress: boolean;
  completeIntialSyncEntries: SyncEntryName[] | string[];
  initialSyncDone: boolean;
};

export type MlSyncState = {
  entryStatus: Partial<SyncMlEntryStatus>;
};

export type SyncStatusMessage = {
  type: 'sync_status';
  value: Omit<SyncState, 'entryStatus'>;
};

export type SyncEntryMessage = {
  type: 'sync_entry';
  value: {
    entry: SyncEntryName;
    state: SyncProgress;
  };
};

export type SyncMlEntryMessage = {
  type: 'sync_ml_entry';
  value: {
    entry: string;
    state: SyncProgress;
  };
};

export type ServiceName = 'db' | 'ipfs' | 'sync' | 'ml' | 'rune';

export type ServiceStatusMessage = {
  type: 'service_status';
  value: { name: ServiceName; status: ServiceStatus; message?: string };
};

export const BC_MSG_LOAD_COMMUNITY = 'load_community';

export type LoadCommunityMessage = {
  type: typeof BC_MSG_LOAD_COMMUNITY;
  value: SyncCommunityResult;
};

export const BC_MSG_SET_DEFAULT_ACCOUNT = setDefaultAccount.type;

export type SetDefaultAccountMessage = typeof setDefaultAccount;

// export type SenseListUpdate = {
//   type: 'sense_list_update';
//   list: SenseListItem[];
// };

export type BroadcastChannelMessage =
  | SyncStatusMessage
  | SyncEntryMessage
  | IndexedDbWriteMessage
  | ServiceStatusMessage
  | LoadCommunityMessage
  | SyncMlEntryMessage
  | SetDefaultAccountMessage;
// | SenseListUpdate
// | SenseListRemove;

export const getBroadcastChannemMessageKey = (msg: BroadcastChannelMessage) => {
  const { type, value } = msg;
  switch (type) {
    case 'service_status':
      return `${type}_${value.name}`;
    case 'sync_entry':
      return `${type}_${value.entry}`;
    case 'sync_ml_entry':
      return `${type}_${value.entry}`;
    default:
      return type;
  }
};
