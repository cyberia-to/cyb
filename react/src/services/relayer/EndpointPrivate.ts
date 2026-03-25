/* eslint-disable no-underscore-dangle */

import {
  AckWithMetadata,
  Endpoint,
  PacketWithMetadata,
  QueryOpts,
} from '@confio/relayer/build/lib/endpoint';
import {
  parseAcksFromLogs,
  parsePacketsFromBlockResult,
  parsePacketsFromLogs,
} from '@confio/relayer/build/lib/utils';
import { toHex } from '@cosmjs/encoding';
import { logs } from '@cosmjs/stargate';

// @ts-expect-error
class EndpointPrivate extends Endpoint {
  _packetSender?: string;

  _counterpartyPacketMinHeight?: number;

  _loadCounterpartyPackets?: (opts: QueryOpts) => Promise<PacketWithMetadata[]>;

  setPacketSender(sender: string) {
    this._packetSender = sender;
  }

  setCounterpartyPacketMinHeight(height: number) {
    this._counterpartyPacketMinHeight = height;
  }

  setLoadCounterpartyPackets(
    loadCounterpartyPackets: (opts: QueryOpts) => Promise<PacketWithMetadata[]>
  ) {
    this._loadCounterpartyPackets = loadCounterpartyPackets;
  }

  private get packetSender(): string {
    if (!this._packetSender) {
      throw new Error('packetSender not set');
    }
    return this._packetSender;
  }

  private get counterpartyPacketMinHeight(): number {
    if (this._counterpartyPacketMinHeight === undefined) {
      throw new Error('counterpartyPacketMinHeight not set');
    }
    return this._counterpartyPacketMinHeight;
  }

  private get loadCounterpartyPackets(): (opts: QueryOpts) => Promise<PacketWithMetadata[]> {
    if (!this._loadCounterpartyPackets) {
      throw new Error('loadCounterpartyPackets not set');
    }
    return this._loadCounterpartyPackets;
  }

  private async getPacketsFromBlockEvents({
    minHeight,
    maxHeight,
  }: QueryOpts = {}): Promise<PacketWithMetadata[]> {
    let query = `message.sender='${this.packetSender}' AND send_packet.packet_connection='${this.connectionID}'`;
    if (minHeight) {
      query = `${query} AND block.height>=${minHeight}`;
    }
    if (maxHeight) {
      query = `${query} AND block.height<=${maxHeight}`;
    }
    const search = await this.client.tm.blockSearchAll({ query });
    console.debug('blocks', search.blocks);
    const resultsNested = await Promise.all(
      search.blocks.map(async ({ block }) => {
        const { height } = block.header;
        const result = await this.client.tm.blockResults(height);
        return parsePacketsFromBlockResult(result).map((packet) => ({
          packet,
          height,
          sender: '',
        }));
      })
    );

    return ([] as PacketWithMetadata[]).concat(...resultsNested);
  }

  async querySentPackets({ minHeight, maxHeight }: QueryOpts = {}): Promise<PacketWithMetadata[]> {
    const packetsFromTxs = await this.getPacketsFromTxs({
      minHeight,
      maxHeight,
    });
    const packetsFromBlockEvents = await this.getPacketsFromBlockEvents({
      minHeight,
      maxHeight,
    });
    const packets = packetsFromTxs.concat(packetsFromBlockEvents);
    return packets;
  }

  private async getPacketsFromTxs({
    minHeight,
    maxHeight,
  }: QueryOpts = {}): Promise<PacketWithMetadata[]> {
    let query = `message.sender='${this.packetSender}' AND send_packet.packet_connection='${this.connectionID}'`;
    if (minHeight) {
      query = `${query} AND tx.height>=${minHeight}`;
    }
    if (maxHeight) {
      query = `${query} AND tx.height<=${maxHeight}`;
    }
    const search = await this.client.tm.txSearchAll({ query });
    console.debug('txs', search.txs);
    const resultsNested = search.txs.map(({ hash, height, result }) => {
      const parsedLogs = logs.parseRawLog(result.log);
      // we accept message.sender (cosmos-sdk) and message.signer (x/wasm)
      let sender = '';
      try {
        sender = logs.findAttribute(parsedLogs, 'message', 'sender').value;
      } catch {
        try {
          sender = logs.findAttribute(parsedLogs, 'message', 'signer').value;
        } catch {
          this.client.logger.warn(`No message.sender nor message.signer in tx ${toHex(hash)}`);
        }
      }
      return parsePacketsFromLogs(parsedLogs).map((packet) => ({
        packet,
        height,
        sender,
      }));
    });
    console.debug(resultsNested);
    return ([] as PacketWithMetadata[]).concat(...resultsNested);
  }

  // returns all acks (auto-paginates, so be careful about not setting a minHeight)
  public async queryWrittenAcks({
    minHeight,
    maxHeight,
  }: QueryOpts = {}): Promise<AckWithMetadata[]> {
    const packets = await this.loadCounterpartyPackets({
      minHeight: this.counterpartyPacketMinHeight,
    });
    let allAcks: AckWithMetadata[] = [];
    console.debug({ ackPackets: packets });
    await Promise.all(
      packets.map(async (p) => {
        // query private packets one at a time. This is a bit inefficient, but
        // still much faster than querying all packets from everyone at once
        let query =
          `write_acknowledgement.packet_connection='${this.connectionID}' AND ` +
          `write_acknowledgement.packet_sequence=${p.packet.sequence.toString()}`;
        if (minHeight) {
          query = `${query} AND tx.height>=${minHeight}`;
        }
        if (maxHeight) {
          query = `${query} AND tx.height<=${maxHeight}`;
        }
        console.debug({ query });
        const search = await this.client.tm.txSearchAll({ query });
        const resultsNested = search.txs.map(({ height, result }) => {
          const parsedLogs = logs.parseRawLog(result.log);
          // const sender = logs.findAttribute(parsedLogs, 'message', 'sender').value;
          return parseAcksFromLogs(parsedLogs).map((ack) => ({
            height,
            ...ack,
          }));
        });

        allAcks = allAcks.concat(...resultsNested);
      })
    );

    this.client.logger.info(
      `Found ${allAcks.length} acks for ${packets.length} packets from ${this.packetSender}`
    );
    return allAcks;
  }
}

export default EndpointPrivate;
