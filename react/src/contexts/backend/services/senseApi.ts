import { Coin } from 'cosmjs-types/cosmos/base/v1beta1/coin';
import { CID_TWEET } from 'src/constants/app';
import { isParticle } from 'src/features/particle/utils';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';
import {
  CYBER_LINK_TRANSACTION_TYPE,
  CyberLinkValue,
  MSG_SEND_TRANSACTION_TYPE,
  MsgSendValue,
} from 'src/services/backend/services/indexer/types';
import { SENSE_FRIEND_PARTICLES } from 'src/services/backend/services/sync/services/consts';
import { syncMyChats } from 'src/services/backend/services/sync/services/SyncTransactionsLoop/services/chat';
import { changeParticleSyncStatus } from 'src/services/backend/services/sync/utils';
import { LinkDto, SyncStatusDto, TransactionDto } from 'src/services/CozoDb/types/dto';
import { EntryType } from 'src/services/CozoDb/types/entities';
import { NeuronAddress, ParticleCid, TransactionHash } from 'src/types/base';
import { EntityToDto } from 'src/types/dto';
import { getNowUtcNumber } from 'src/utils/date';

type LocalSenseChatMessage = {
  transactionHash: TransactionHash;
  fromAddress: NeuronAddress;
  toAddress: NeuronAddress;
  amount: Coin[];
  memo: string;
};

const prepareSenseMsgSendTransaction = ({
  transactionHash,
  fromAddress,
  toAddress,
  amount,
  memo,
}: LocalSenseChatMessage) => {
  const value = {
    fromAddress,
    toAddress,
    amount,
  } as EntityToDto<MsgSendValue>;

  const transaction = {
    hash: transactionHash,
    type: MSG_SEND_TRANSACTION_TYPE,
    index: 0,
    timestamp: getNowUtcNumber(),
    success: true,
    value,
    memo,
    neuron: fromAddress,
    blockHeight: -1,
  } as TransactionDto;

  return transaction;
};

const prepareSenseCyberlinkTransaction = (link: LinkDto) => {
  const { from, to, neuron, transactionHash, timestamp } = link;
  const value = {
    neuron,
    links: [{ from, to }],
  } as EntityToDto<CyberLinkValue>;

  const transaction = {
    hash: transactionHash,
    type: CYBER_LINK_TRANSACTION_TYPE,
    index: 0,
    timestamp,
    success: true,
    value,
    memo: '',
    neuron,
    blockHeight: -1,
  } as TransactionDto;

  return transaction;
};

export const createSenseApi = (
  dbApi: DbApiWrapper,
  myAddress?: NeuronAddress,
  followingAddresses = [] as NeuronAddress[]
) => ({
  getList: async () => {
    const result = await dbApi.getSenseList(myAddress);
    // console.log(
    //   '--- getList unread',
    //   result.filter((r) => r.unreadCount > 0)
    // );
    return result;
  },
  markAsRead: async (id: NeuronAddress | ParticleCid, lastTimestampRead?: number) => {
    console.time(`--- senseMarkAsRead done ${id}`);
    const syncItem = await dbApi.getSyncStatus(myAddress!, id);

    let unreadCount = 0;
    let timestampRead = syncItem.timestampUpdate;

    if (lastTimestampRead) {
      timestampRead = lastTimestampRead;

      if (isParticle(id)) {
        const links = await dbApi.getLinks({
          cid: id,
          neuron: myAddress!,
          timestampFrom: lastTimestampRead,
        });

        unreadCount = links.length;
      } else {
        const links = await dbApi.getLinks({
          cid: SENSE_FRIEND_PARTICLES,
          neuron: id,
          timestampFrom: lastTimestampRead,
        });

        const chats = await dbApi.getMyChats(myAddress!, id);

        unreadCount = links.length + chats.length;
      }
    }

    const res = await dbApi.updateSyncStatus({
      id,
      ownerId: myAddress!,
      timestampRead, //  conditional or read all
      unreadCount,
    });
    console.log('---  - senseMarkAsReadres', syncItem, res, timestampRead);
    console.timeEnd(`--- senseMarkAsRead done ${id}`);
    return res;
  },
  getAllParticles: (fields: string[]) => dbApi.getParticlesRaw(fields),
  getLinks: (cid: ParticleCid) => dbApi.getLinks({ cid }),
  addMsgSendAsLocal: async (msg: LocalSenseChatMessage) => {
    /*
    This function create syntetic transaction (blockHeight=-1)
    and create syncItem as well.
    */
    const transaction = prepareSenseMsgSendTransaction(msg);

    await dbApi.putTransactions([transaction]);
    const items = await syncMyChats(
      dbApi,
      myAddress!,
      transaction.timestamp,
      new AbortController().signal,
      false
    );
    new BroadcastChannelSender().postSenseUpdate(items);
  },
  addCyberlinkLocal: async (link: LinkDto) => {
    const syncItemLinkTo = await dbApi.getSyncStatus(myAddress!, link.to);
    const syncItemLinkFrom = await dbApi.getSyncStatus(myAddress!, link.from);
    let syncItemLink;

    if (link.from === CID_TWEET) {
      syncItemLink = syncItemLinkTo.id
        ? syncItemLinkTo
        : ({
            ownerId: myAddress!,
            id: link.to,
            unreadCount: 0,
            timestampRead: 0,
            timestampUpdate: 0,
            meta: { ...link },
            entryType: EntryType.particle,
            disabled: false,
          } as SyncStatusDto);
    } else {
      syncItemLink = syncItemLinkFrom.id ? syncItemLinkFrom : syncItemLinkTo;
    }

    // Just cyberlink, no tweet no syncable item
    if (!syncItemLink || !syncItemLink.id) {
      return;
    }

    const transaction = prepareSenseCyberlinkTransaction(link);
    await dbApi.putTransactions([transaction]);
    const newItem = changeParticleSyncStatus(syncItemLink, [link], myAddress!, false);
    await dbApi.putSyncStatus(newItem);
    new BroadcastChannelSender().postSenseUpdate([newItem]);
  },
  putCyberlink: (links: LinkDto | LinkDto[]) => dbApi.putCyberlinks(links),
  getTransactions: (neuron: NeuronAddress) => dbApi.getTransactions(neuron),
  getFriendItems: async (userAddress: NeuronAddress) => {
    if (!myAddress) {
      throw new Error('myAddress is not defined');
    }
    const chats = await dbApi.getMyChats(myAddress, userAddress);
    const links = followingAddresses.includes(userAddress)
      ? await dbApi.getLinks({
          neuron: userAddress,
          cid: SENSE_FRIEND_PARTICLES,
        })
      : [];

    // merge 2 lists and reorder
    const result = [...chats, ...links].sort((a, b) => (a.timestamp > b.timestamp ? 1 : -1));

    return result;
  },
});

export type SenseApi = ReturnType<typeof createSenseApi> | null;
