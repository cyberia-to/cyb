import { BroadcastChannelMessage } from '../types/services';
import { CYB_BROADCAST_CHANNEL } from './consts';

class BroadcastChannelListener {
  private channel: BroadcastChannel;

  constructor(onMessage?: (msg: MessageEvent<BroadcastChannelMessage>) => void) {
    this.channel = new BroadcastChannel(CYB_BROADCAST_CHANNEL);

    if (onMessage) {
      this.channel.onmessage = (event) => onMessage(event);
    }
  }
  close() {
    this.channel.close();
  }
}

export default BroadcastChannelListener;
