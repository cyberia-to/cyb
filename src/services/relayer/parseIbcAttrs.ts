import { findAttribute } from '@cosmjs/stargate/build/logs';
import { TxResponse } from '@cosmjs/tendermint-rpc';

type PacketAttrs = {
  seq: number;
  srcChannel: string;
  srcPort: string;
  destChannel: string;
  destPort: string;
};

function parseIbcAttrs(tx: TxResponse): PacketAttrs {
  // sample ibc tx: https://www.mintscan.io/osmosis/txs/680A76DE87A40401574CDE66941775B832B20DAB8062CFF66DCDAF668E20BADA
  const seq = findAttribute(JSON.parse(tx.result.log!), 'send_packet', 'packet_sequence');
  const srcChannel = findAttribute(JSON.parse(tx.result.log!), 'send_packet', 'packet_src_channel');
  const srcPort = findAttribute(JSON.parse(tx.result.log!), 'send_packet', 'packet_src_port');
  const destChannel = findAttribute(
    JSON.parse(tx.result.log!),
    'send_packet',
    'packet_dst_channel'
  );
  const destPort = findAttribute(JSON.parse(tx.result.log!), 'send_packet', 'packet_dst_port');
  return {
    seq: parseInt(seq.value, 10),
    srcChannel: srcChannel.value,
    srcPort: srcPort.value,
    destChannel: destChannel.value,
    destPort: destPort.value,
  };
}

export default parseIbcAttrs;
