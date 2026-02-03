import { Event } from '@cosmjs/stargate';

const parseEvents = (events: readonly Event[]) => {
  try {
    if (events && !events.length) {
      return undefined;
    }
    // eslint-disable-next-line no-restricted-syntax
    for (const event of events) {
      if (event.type === 'send_packet') {
        const { attributes } = event;
        const sourceChannelAttr = attributes.find(
          (attr) => attr.key === 'packet_src_channel'
        );
        const sourceChannelValue = sourceChannelAttr
          ? sourceChannelAttr.value
          : undefined;
        const destChannelAttr = attributes.find(
          (attr) => attr.key === 'packet_dst_channel'
        );
        const destChannelValue = destChannelAttr
          ? destChannelAttr.value
          : undefined;
        const sequenceAttr = attributes.find(
          (attr) => attr.key === 'packet_sequence'
        );
        const sequence = sequenceAttr ? sequenceAttr.value : undefined;
        const timeoutHeightAttr = attributes.find(
          (attr) => attr.key === 'packet_timeout_height'
        );
        const timeoutHeight = timeoutHeightAttr
          ? timeoutHeightAttr.value
          : undefined;
        const timeoutTimestampAttr = attributes.find(
          (attr) => attr.key === 'packet_timeout_timestamp'
        );
        const timeoutTimestamp = timeoutTimestampAttr
          ? timeoutTimestampAttr.value
          : undefined;
        if (sequence && destChannelValue && sourceChannelValue) {
          return {
            destChannelId: destChannelValue,
            sourceChannelId: sourceChannelValue,
            sequence,
            timeoutHeight,
            timeoutTimestamp,
          };
        }
      }
    }
  } catch (e) {
    console.debug('error parseLog', e);
    return null;
  }
};

export default parseEvents;
