/* eslint-disable no-restricted-syntax */
import { QueryClient, setupIbcExtension } from '@cosmjs/stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import { IdentifiedConnection } from 'cosmjs-types/ibc/core/connection/v1/connection';

export interface MatchingCxn {
  cxnA: string;
  cxnB: string;
  arbitraryPriority: number;
}
async function loadMatchingConnections(
  tmA: Tendermint34Client,
  tmB: Tendermint34Client
): Promise<readonly MatchingCxn[]> {
  const qcA = QueryClient.withExtensions(tmA, setupIbcExtension);
  const qcB = QueryClient.withExtensions(tmB, setupIbcExtension);
  const cxnsA = await qcA.ibc.connection.allConnections();
  const cxnsB = await qcB.ibc.connection.allConnections();
  console.debug('loaded connections');
  // const cxnsAByCounterpartyCxnId = new Map(
  //   cxnsA.connections
  //     .filter((cxn) => !!cxn.counterparty)
  //     .map((cxn) => [cxn.counterparty!.connectionId, cxn])
  // );
  // const cxnPairs = cxnsB.connections
  //   .map((cxnB) => {
  //     const cxnA = cxnsAByCounterpartyCxnId.get(cxnB.id);
  //     if (cxnA && cxnA?.id == cxnB.counterparty?.connectionId) {
  //       return { cxnA: cxnA.id, cxnB: cxnB.id };
  //     }
  //   })
  //   .filter((cxn) => !!cxn);
  const cxnPairs: { cxnA: IdentifiedConnection; cxnB: IdentifiedConnection }[] = [];
  for (const cxnA of cxnsA.connections) {
    for (const cxnB of cxnsB.connections) {
      if (
        cxnA.counterparty?.connectionId === cxnB.id &&
        cxnB.counterparty?.connectionId === cxnA.id
      ) {
        cxnPairs.push({ cxnA, cxnB });
      }
    }
  }
  const matchingCxns: MatchingCxn[] = [];
  await Promise.all(
    cxnPairs.map(async (cxnPair) => {
      const { cxnA } = cxnPair!;
      const { cxnB } = cxnPair!;
      // const chansA = await qcA.ibc.channel.allConnectionChannels(cxnA);
      // const chansB = await qcB.ibc.channel.allConnectionChannels(cxnB);
      // console.log(chansA, chansB);
      // console.log(
      //   `Found ${chansA.channels.length} channels on ${cxnA} and ${chansB.channels.length} channels on ${cxnB}`
      // );
      // console.log(
      //   `Checking ${
      //     chansA.channels.length * chansB.channels.length
      //   } channel pairs`
      // );

      // for (const chanA of chansA.channels) {
      //   const acksA = await qcA.ibc.channel.packetAcknowledgements(
      //     chanA.portId,
      //     chanA.channelId
      //   );
      //   totalPriority += acksA.pagination?.total.toNumber() || 0;
      // }
      // for (const chanB of chansB.channels) {
      //   const acksB = await qcB.ibc.channel.packetAcknowledgements(
      //     chanB.portId,
      //     chanB.channelId
      //   );
      //   totalPriority += acksB.pagination?.total.toNumber() || 0;
      // }
      // load proof height for each connection
      let arbitraryPriority = 0;
      const shouldLoadArbitraryPriority = false;
      if (shouldLoadArbitraryPriority) {
        const {
          txs: [txA],
        } = await tmA.txSearch({
          query: `update_client.client_id='${cxnA.clientId}'`,
          per_page: 1,
          order_by: 'desc',
        });
        const {
          txs: [txB],
        } = await tmB.txSearch({
          query: `update_client.client_id='${cxnB.clientId}'`,
          per_page: 1,
          order_by: 'desc',
        });
        const lastBlockTime = Math.max(
          txA && txA.height > 0
            ? await tmA.block(txA.height).then((b) => b.block.header.time.getTime())
            : 0,
          txB && txB.height > 0
            ? await tmB.block(txB.height).then((b) => b.block.header.time.getTime())
            : 0
        );
        arbitraryPriority = lastBlockTime;
      }
      matchingCxns.push({
        cxnA: cxnA.id,
        cxnB: cxnB.id,
        arbitraryPriority,
      });
    })
  );
  return matchingCxns.sort(
    (a, b) => b.arbitraryPriority - a.arbitraryPriority
  ) as readonly MatchingCxn[];
}

export default loadMatchingConnections;
