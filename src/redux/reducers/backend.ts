import { clone } from 'lodash';
import { assocPath } from 'ramda';
import { SYNC_ENTRIES_TO_TRACK_PROGRESS } from 'src/services/backend/services/sync/services/consts';
import { syncEntryNameToReadable } from 'src/services/backend/services/sync/utils';
import {
  BroadcastChannelMessage,
  MlSyncState,
  ServiceName,
  ServiceStatus,
  SyncState,
} from 'src/services/backend/types/services';
import { CommunityDto } from 'src/services/CozoDb/types/dto';
import { NeuronAddress } from 'src/types/base';
import { removeDublicates } from 'src/utils/list';

export const RESET_SYNC_STATE_ACTION_NAME = 'reset_sync_entry';

type BackendState = {
  dbPendingWrites: number;
  syncState: SyncState;
  mlState: MlSyncState;
  community: {
    isLoaded: boolean;
    raw: CommunityDto[];
    following: NeuronAddress[];
    followers: NeuronAddress[];
    friends: NeuronAddress[];
  };
  services: {
    [key in ServiceName]: {
      status: ServiceStatus;
      error?: string;
      message?: string;
    };
  };
};

const initialSyncState = {
  // status: 'inactive',
  entryStatus: {},
  totalEstimatedTime: 0,
  message: '',
  inProgress: false,
  completeIntialSyncEntries: [],
  initialSyncDone: false,
};

const initialState: BackendState = {
  dbPendingWrites: 0,
  community: {
    isLoaded: false,
    raw: [],
    following: [],
    followers: [],
    friends: [],
  },
  syncState: initialSyncState,
  mlState: { entryStatus: {} },
  services: {
    db: { status: 'inactive' },
    ipfs: { status: 'inactive' },
    sync: { status: 'inactive' },
    ml: { status: 'inactive' },
    rune: { status: 'inactive' },
  },
};

// Backend state
function backendReducer(
  state = initialState,
  action:
    | BroadcastChannelMessage
    | {
        type: typeof RESET_SYNC_STATE_ACTION_NAME;
      }
) {
  switch (action.type) {
    case 'indexeddb_write': {
      return assocPath(['dbPendingWrites'], action.value, state);
    }

    case RESET_SYNC_STATE_ACTION_NAME: {
      return { ...state, syncState: initialSyncState };
    }
    case 'sync_entry': {
      const { entry, state: entryState } = action.value;
      const updatedEntry = {
        ...state.syncState.entryStatus[entry],
        ...entryState,
      };
      const newState = assocPath(['syncState', 'entryStatus', entry], updatedEntry, state);

      const messages: string[] = [];
      let totalEstimatedTime = 0;
      const { entryStatus, completeIntialSyncEntries } = newState.syncState;
      const newCompleteIntitalSyncEntries = clone(completeIntialSyncEntries);
      SYNC_ENTRIES_TO_TRACK_PROGRESS.forEach((name) => {
        if (entryStatus[name]) {
          const { progress, status } = newState.syncState.entryStatus[name]!;

          // Push flags that initial sync was done
          if (
            status &&
            ['active', 'listen'].some((s) => s === status) &&
            !completeIntialSyncEntries.some((entryName) => entryName === name)
          ) {
            newCompleteIntitalSyncEntries.push(name);
          }

          if (progress && status === 'in-progress') {
            totalEstimatedTime += progress.estimatedTime;
            const percents = Math.round((progress.completeCount / progress.totalCount) * 100);
            messages.push(`${syncEntryNameToReadable(name)}: ${percents}%`);
          }
        }
      });

      const initialSyncDone = SYNC_ENTRIES_TO_TRACK_PROGRESS.reduce(
        (prev, curr) => prev && newCompleteIntitalSyncEntries.some((n) => n === curr),
        true
      );

      newState.syncState = {
        ...newState.syncState,
        message: messages.join(', '),
        totalEstimatedTime,
        inProgress: messages.length > 0,
        completeIntialSyncEntries: newCompleteIntitalSyncEntries,
        initialSyncDone,
      };

      return newState;
    }

    case 'service_status': {
      const { name, status, message } = action.value;
      return assocPath(['services', name], { status, message }, state);
    }

    case 'load_community': {
      const { action: stateAction, items } = action.value;
      // console.log('------------load_community', stateAction, items, state);
      if (stateAction === 'reset') {
        const community = {
          isLoaded: false,
          raw: [],
          following: [],
          followers: [],
          friends: [],
        };
        return { ...state, community };
      }
      const allItems = [...state.community.raw, ...items];

      const community = {
        isLoaded: stateAction === 'complete',
        raw: allItems,
        following: removeDublicates(
          allItems.filter((item) => item.following && !item.follower).map((item) => item.neuron)
        ),
        followers: removeDublicates(
          allItems.filter((item) => item.follower && !item.following).map((item) => item.neuron)
        ),
        friends: removeDublicates(
          allItems.filter((item) => item.follower && item.following).map((item) => item.neuron)
        ),
      };

      return { ...state, community };
    }

    case 'sync_ml_entry': {
      const { entry, state: entryState } = action.value;

      // console.log('------sync_ml_entry', action, entryState);
      const newState = assocPath(['mlState', 'entryStatus', entry], entryState, state);
      return newState;
    }

    default:
      return state;
  }
}

export default backendReducer;
