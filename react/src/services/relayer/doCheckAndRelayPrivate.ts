import { Link as IbcLink, Logger } from '@confio/relayer/build';
import { AckWithMetadata } from '@confio/relayer/build/lib/endpoint';
import { RelayedHeights } from '@confio/relayer/build/lib/link';
import { secondsFromDateNanos, splitPendingPackets } from '@confio/relayer/build/lib/utils';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import EndpointPrivate from './EndpointPrivate';
import queryPacketAttrsBySender from './queryPacketAttrsBySender';

type RelayInfo = {
  packetsFromA: number;
  packetsFromB: number;
  acksFromA: AckWithMetadata[];
  acksFromB: AckWithMetadata[];
};

async function doCheckAndRelayPrivate(
  link: IbcLink,
  relayFrom: RelayedHeights,
  senderA: string,
  senderB: string,
  tmA: Tendermint34Client,
  tmB: Tendermint34Client,
  timedoutThresholdBlocks = 0,
  timedoutThresholdSeconds = 0,
  logger: Logger
): Promise<{ heights: RelayedHeights; info: RelayInfo }> {
  const privateEndA = new EndpointPrivate(
    link.endA.client,
    link.endA.clientID,
    link.endA.connectionID
  );
  privateEndA.setPacketSender(senderA);
  privateEndA.setCounterpartyPacketMinHeight(relayFrom.packetHeightB || 0);
  const privateEndB = new EndpointPrivate(
    link.endB.client,
    link.endB.clientID,
    link.endB.connectionID
  );
  privateEndB.setPacketSender(senderB);
  privateEndB.setCounterpartyPacketMinHeight(relayFrom.packetHeightA || 0);

  const querySentPacketsB = privateEndB.querySentPackets.bind(privateEndB);
  privateEndA.setLoadCounterpartyPackets((opts = {}) => {
    opts.minHeight = relayFrom.packetHeightB || 0;
    return querySentPacketsB(opts);
  });
  const querySentPacketsA = privateEndA.querySentPackets.bind(privateEndA);
  privateEndB.setLoadCounterpartyPackets((opts = {}) => {
    opts.minHeight = relayFrom.packetHeightA || 0;
    return querySentPacketsA(opts);
  });
  // @ts-expect-error
  link.endA = privateEndA;
  // @ts-expect-error
  link.endB = privateEndB;

  console.debug('Relaying from', relayFrom);
  const { ibcAttrs: senderPacketsA } = await queryPacketAttrsBySender(
    tmA,
    link.endA.connectionID,
    senderA,
    relayFrom.packetHeightA || 0
  );
  console.debug('Sender packets A', senderPacketsA);
  const { ibcAttrs: senderPacketsB } = await queryPacketAttrsBySender(
    tmB,
    link.endB.connectionID,
    senderB,
    relayFrom.packetHeightB || 0
  );

  console.debug('Sender packets B', senderPacketsB);
  // FIXME: is there a cleaner way to get the height we query at?
  const [packetHeightA, packetHeightB, packetsA, packetsB] = await Promise.all([
    link.endA.client.currentHeight(),
    link.endB.client.currentHeight(),
    link.getPendingPackets('A', { minHeight: relayFrom.packetHeightA }).catch((e) => {
      console.error('Error getting pending packets A', e);
      return [];
    }),
    link.getPendingPackets('B', { minHeight: relayFrom.packetHeightB }).catch((e) => {
      console.error('Error getting pending packets B', e);
      return [];
    }),
  ]);

  console.debug('Packets A', packetsA);
  console.debug('Packets B', packetsB);

  const cutoffHeightA = await link.endB.client.timeoutHeight(timedoutThresholdBlocks);
  const cutoffTimeA =
    secondsFromDateNanos(await link.endB.client.currentTime()) + timedoutThresholdSeconds;
  const { toSubmit: submitA, toTimeout: timeoutA } = splitPendingPackets(
    cutoffHeightA,
    cutoffTimeA,
    packetsA
  );

  const cutoffHeightB = await link.endA.client.timeoutHeight(timedoutThresholdBlocks);
  const cutoffTimeB =
    secondsFromDateNanos(await link.endA.client.currentTime()) + timedoutThresholdSeconds;
  const { toSubmit: submitB, toTimeout: timeoutB } = splitPendingPackets(
    cutoffHeightB,
    cutoffTimeB,
    packetsB
  );

  console.debug('Submitting A & B', submitA, submitB);
  // FIXME: use the returned acks first? Then query for others?
  await Promise.all([link.relayPackets('A', submitA), link.relayPackets('B', submitB)]).catch(
    (e) => {
      console.error('Error relaying packets', e);
      logger.error('Error relaying packets', e);
    }
  );

  console.debug('Waiting for indexer');
  // let's wait a bit to ensure our newly committed acks are indexed
  await Promise.all([link.endA.client.waitForIndexer(), link.endB.client.waitForIndexer()]);

  console.debug('Getting acks');
  const [ackHeightA, ackHeightB, acksARaw, acksBRaw] = await Promise.all([
    link.endA.client.currentHeight(),
    link.endB.client.currentHeight(),
    link.getPendingAcks('A', { minHeight: relayFrom.ackHeightA }),
    link.getPendingAcks('B', { minHeight: relayFrom.ackHeightB }),
  ]);

  const acksA = acksARaw;
  const acksB = acksBRaw;

  // const acksA = acksARaw.filter((a) => {
  //   const packet = a.originalPacket;
  //   const key = `${packet.sourcePort}/${packet.sourceChannel}/${packet.sequence}`;
  //   return senderPacketsA.some((p) => {
  //     const pKey = `${p.srcPort}/${p.srcChannel}/${p.seq}`;
  //     return pKey === key;
  //   });
  // });

  // const acksB = acksBRaw.filter((a) => {
  //   const packet = a.originalPacket;
  //   const key = `${packet.sourcePort}/${packet.sourceChannel}/${packet.sequence}`;
  //   return senderPacketsB.some((p) => {
  //     const pKey = `${p.srcPort}/${p.srcChannel}/${p.seq}`;
  //     return pKey === key;
  //   });
  // });

  console.debug('Relaying acks A & B', acksA, acksB);

  await Promise.all([link.relayAcks('A', acksA), link.relayAcks('B', acksB)]).catch((e) => {
    logger.error(`Error relaying acks${JSON.stringify(e)}`);
  });

  console.debug('timing out packets A & B', timeoutA, timeoutB);
  await Promise.all([link.timeoutPackets('A', timeoutA), link.timeoutPackets('B', timeoutB)]).catch(
    (e) => {
      logger.error('Error timing out packets', e);
    }
  );

  const heights = {
    packetHeightA,
    packetHeightB,
    ackHeightA,
    ackHeightB,
  };

  const info = {
    packetsFromA: packetsA.length,
    packetsFromB: packetsB.length,
    acksFromA: acksA,
    acksFromB: acksB,
  };

  return { heights, info };
}

export default doCheckAndRelayPrivate;
