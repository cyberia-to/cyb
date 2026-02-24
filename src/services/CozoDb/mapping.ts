import { LsResult } from 'ipfs-core-types/src/pin';
import { CyberlinksByParticleQuery } from 'src/generated/graphql';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { dateToUtcNumber } from 'src/utils/date';
import { removeMarkdownFormatting, replaceQuotes } from 'src/utils/string';
import { Transaction } from '../backend/services/indexer/types';
import { IPFSContent } from '../ipfs/types';
import { LinkDto, ParticleDto, PinDto, TransactionDto } from './types/dto';
import { PinTypeMap } from './types/entities';

export const mapParticleToEntity = (particle: IPFSContent): ParticleDto => {
  const { cid, meta, textPreview } = particle;
  const { size, mime, type, blocks, sizeLocal } = meta;

  // hack to fix string command
  const text = textPreview ? replaceQuotes(removeMarkdownFormatting(textPreview)) : '';

  return {
    cid,
    size: size || 0,
    mime: mime || 'unknown',
    type,
    text,
    size_local: sizeLocal || -1,
    blocks: blocks || 0,
  };
};

//TODO: REFACTOR
export const mapPinToEntity = (pin: LsResult): PinDto => ({
  cid: pin.cid.toString(),
  type: PinTypeMap[pin.type],
});

export const mapIndexerTransactionToEntity = (neuron: string, tx: Transaction): TransactionDto => {
  const {
    transaction_hash,
    index,
    transaction: {
      memo,
      block: { timestamp, height },
      success,
    },
    type,
    value,
  } = tx;
  return {
    hash: transaction_hash,
    index,
    type,
    timestamp: dateToUtcNumber(timestamp),
    // value: JSON.stringify(value),
    memo,
    value,
    success,
    neuron,
    blockHeight: height,
  };
};

// export const mapSyncStatusToEntity = (
//   id: NeuronAddress | ParticleCid,
//   entryType: EntryType,
//   unreadCount: number,
//   timestampUpdate: number,
//   lastId: TransactionHash | ParticleCid = '',
//   timestampRead: number = timestampUpdate,
//   meta: Object = {}
// ): SyncStatusDbEntity => {
//   return {
//     entry_type: entryType,
//     id,
//     timestamp_update: timestampUpdate,
//     timestamp_read: timestampRead,
//     unread_count: unreadCount,
//     disabled: false,
//     last_id: lastId,
//     meta,
//   };
// };

export const mapLinkToLinkDto = (
  from: ParticleCid,
  to: ParticleCid,
  neuron: NeuronAddress = '',
  timestamp: number = 0
): LinkDto => ({
  from,
  to,
  neuron,
  timestamp,
});

export const mapLinkFromIndexerToDto = ({
  from,
  to,
  neuron,
  timestamp,
  transaction_hash,
}: CyberlinksByParticleQuery['cyberlinks'][0]): LinkDto => ({
  from,
  to,
  neuron,
  timestamp: dateToUtcNumber(timestamp),
  transactionHash: transaction_hash,
});
