import { Tendermint34Client, TxSearchParams } from '@cosmjs/tendermint-rpc';
import parseIbcAttrs from './parseIbcAttrs';

async function queryPacketAttrsBySender(
  tm: Tendermint34Client,
  cxn: string,
  sender: string,
  startHeight: number,
  params: Partial<TxSearchParams> = {}
) {
  const { txs, totalCount } = await tm.txSearchAll({
    query:
      `message.sender='${sender}' AND ` +
      `send_packet.packet_connection='${cxn}' AND ` +
      `tx.height>=${startHeight}`,
    ...params,
  });
  return {
    ibcAttrs: txs.map(parseIbcAttrs),
    totalCount,
  };
}

export default queryPacketAttrsBySender;
