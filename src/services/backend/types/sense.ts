/* eslint-disable import/no-unused-modules */

import { EntryType } from 'src/services/CozoDb/types/entities';
import { NeuronAddress, ParticleCid, TransactionHash } from 'src/types/base';

import {
  CyberLinkTransaction,
  MsgMultiSendTransaction,
  MsgSendTransaction,
  Transaction,
} from '../services/indexer/types';

export type SenseTransaction = MsgSendTransaction | MsgMultiSendTransaction | CyberLinkTransaction;

export type SenseUnread = {
  entryType: EntryType;
  unreadCount: number;
};

export type SenseLinkMeta = {
  from: ParticleCid;
  to: ParticleCid;
  neuron: NeuronAddress;
  timestamp: number;
  transactionHash: TransactionHash;
};

export type SenseTransactionMeta = {
  transactionHash: TransactionHash;
  index: number;
};

export type SenseListItemTransactionMeta = {
  timestamp: number;
  success: boolean;
  value: Transaction['value'];
  memo?: string;
  hash: TransactionHash;
  index: number;
};

type SenseListItemLinkMeta = SenseLinkMeta;

// Extension for Chat item to separate chat sync from tweet sync
export type SenseChatExtension = {
  timestampUpdateChat?: number;
  timestampUpdateContent?: number;
};

export type SenseItemLinkMeta = SenseLinkMeta & SenseChatExtension;
export type SenseItemTransactionMeta = SenseTransactionMeta & SenseChatExtension;

export type SenseListItemtMeta = SenseListItemLinkMeta | SenseListItemTransactionMeta;

export type SenseListChatItem = {
  entryType: EntryType;
  id: NeuronAddress;
  unreadCount: number;
  timestampUpdate: number;
  timestampRead: number;
  meta: SenseItemLinkMeta | SenseItemTransactionMeta;
};

export type SenseListParticleItem = {
  entryType: EntryType;
  id: ParticleCid;
  unreadCount: number;
  timestampUpdate: number;
  timestampRead: number;
  meta: SenseItemLinkMeta;
};

export type SenseListItem = SenseListParticleItem | SenseListChatItem;
