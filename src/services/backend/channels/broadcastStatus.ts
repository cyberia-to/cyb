import { ProgressTracking, SyncEntryName, SyncProgress } from '../types/services';
import BroadcastChannelSender from './BroadcastChannelSender';

export const broadcastStatus = (name: SyncEntryName, channelApi: BroadcastChannelSender) => {
  // const cyblogCh = createCyblogChannel({ thread: 'bckd', module: name });
  return {
    sendStatus: (status: SyncProgress['status'], message?: string, progress?: ProgressTracking) => {
      // cyblogCh.info(`>>>$ sync ${name} status: ${status} message: ${message}`);
      channelApi.postSyncEntryProgress(name, {
        status,
        message,
        progress,
        done: ['active', 'error', 'listen'].some((s) => s === status),
      });
    },
  };
};
