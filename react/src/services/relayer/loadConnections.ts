/* eslint-disable no-restricted-syntax */
import { QueryClient, setupIbcExtension } from '@cosmjs/stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import { State } from 'cosmjs-types/ibc/core/channel/v1/channel';
import { Channel as ChannelHub } from 'src/types/hub';
import networkList from 'src/utils/networkListIbc';

const PORT_ID = 'transfer';

function findRpc(chainId: string) {
  return networkList[chainId].rpc;
}

async function getConnection(chain: string, channel: string) {
  try {
    const tm = await Tendermint34Client.connect(findRpc(chain));

    const qc = QueryClient.withExtensions(tm, setupIbcExtension);

    const cxns = await qc.ibc.channel.channel(PORT_ID, channel);

    if (!cxns.channel || cxns.channel.state !== State.STATE_OPEN) {
      return undefined;
    }

    return cxns.channel.connectionHops[0];
  } catch (e) {
    console.debug('e', e);
    return undefined;
  }
}

interface MatchingCxn {
  cxnA: string;
  cxnB: string;
}
async function loadConnections(channel: ChannelHub): Promise<readonly MatchingCxn[]> {
  const {
    source_chain_id: chainA,
    destination_chain_id: chainB,
    source_channel_id: channelA,
    destination_channel_id: channelB,
  } = channel;

  const matchingCxns: MatchingCxn[] = [];

  const cxnsA = await getConnection(chainA, channelA);
  const cxnsB = await getConnection(chainB, channelB);

  console.debug('loaded connections');

  if (cxnsA && cxnsB) {
    matchingCxns.push({
      cxnA: cxnsA,
      cxnB: cxnsB,
    });
  }

  return matchingCxns;
}

export default loadConnections;
