import { merge, Observable, Subscription } from 'rxjs';
import { bufferTime, filter } from 'rxjs/operators';
import { AppDispatch } from 'src/redux/store';

import {
  BC_MSG_LOAD_COMMUNITY,
  BC_MSG_SET_DEFAULT_ACCOUNT,
  BroadcastChannelMessage,
  getBroadcastChannemMessageKey,
} from '../types/services';
import { CYB_BROADCAST_CHANNEL } from './consts';

const shouldTrottle = (msg: MessageEvent<BroadcastChannelMessage>) =>
  ['sync_entry', 'sync_status', 'indexeddb_write', 'sync_ml_entry'].some(
    (name) => name === msg.data.type
  );

class RxBroadcastChannelListener {
  private subscription: Subscription;

  constructor(dispatch: AppDispatch) {
    const messageObservable = new Observable<MessageEvent<BroadcastChannelMessage>>(
      (subscriber) => {
        const channel = new BroadcastChannel(CYB_BROADCAST_CHANNEL);

        channel.onmessage = (msg: MessageEvent<BroadcastChannelMessage>) => {
          if (
            msg.data.type === BC_MSG_LOAD_COMMUNITY ||
            msg.data.type === BC_MSG_SET_DEFAULT_ACCOUNT
          ) {
            dispatch(msg.data);
            return;
          }
          subscriber.next(msg);
        };

        return () => {
          channel.onmessage = null;
        };
      }
    );

    const bufferedMessages = messageObservable.pipe(
      filter((m) => shouldTrottle(m)),
      bufferTime(2000)
    ); // Accumulate messages in a 2-second window

    const normalMessages = messageObservable.pipe(
      filter((m) => !shouldTrottle(m)),
      bufferTime(0)
    );

    this.subscription = merge(bufferedMessages, normalMessages)
      .pipe(filter((m) => m.length > 0))
      .subscribe((messages) => {
        if (messages.length > 0) {
          const items = new Map<string, BroadcastChannelMessage>();
          messages.forEach((msg) => {
            const key = getBroadcastChannemMessageKey(msg.data);
            items.set(key, msg.data);
          });
          items.forEach(dispatch);
        }
      });
  }

  close() {
    this.subscription.unsubscribe();
  }
}

export default RxBroadcastChannelListener;
