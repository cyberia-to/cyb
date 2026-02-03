import { Coin, ibc } from 'osmojs';
import defaultNetworks from 'src/constants/defaultNetworks';
import { Channel } from 'src/types/hub';
import { fromBech32 } from 'src/utils/utils';

function newIbcMessage(tokens: Coin[], channel: Channel, sender: string) {
  const { transfer } = ibc.applications.transfer.v1.MessageComposer.withTypeUrl;

  const stamp = Date.now();
  const timeoutInNanos = (stamp + 1.2e6) * 1e6;
  const counterpartyAccount = fromBech32(
    sender,
    defaultNetworks.bostrom.BECH32_PREFIX
  );

  return tokens.map((item) => {
    return transfer({
      sourcePort: 'transfer',
      sourceChannel: channel.destination_channel_id,
      sender,
      token: item,
      receiver: counterpartyAccount,
      timeoutTimestamp: BigInt(timeoutInNanos),
      timeoutHeight: undefined,
      memo: '',
    });
  });
}

export default newIbcMessage;
