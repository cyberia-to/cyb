import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';
import { SenseListItem, SenseTransactionMeta } from 'src/services/backend/types/sense';
import { EntryType } from 'src/services/CozoDb/types/entities';
import { NeuronAddress } from 'src/types/base';
import { throwIfAborted } from 'src/utils/async/promise';
import { extractSenseChats } from '../../utils/sense';

// eslint-disable-next-line import/prefer-default-export
export const syncMyChats = async (
  db: DbApiWrapper,
  myAddress: NeuronAddress,
  timestampFrom: number,
  signal: AbortSignal,
  shouldUpdateTimestamp = true
) => {
  const syncItems = await db.findSyncStatus({
    ownerId: myAddress,
    entryType: EntryType.chat,
  });

  const syncItemsMap = new Map(syncItems?.map((i) => [i.id, i]));

  const myTransactions = await db.getTransactions(myAddress, {
    order: 'asc',
    timestampFrom,
  });

  const myChats = extractSenseChats(myAddress, myTransactions!);

  const results: SenseListItem[] = [];

  // eslint-disable-next-line no-restricted-syntax
  for (const chat of myChats.values()) {
    const syncItem = syncItemsMap.get(chat.userAddress);
    const lastTransaction = chat.transactions.at(-1)!;

    const { timestamp: transactionTimestamp, hash, index } = lastTransaction;
    const syncItemHeader = {
      entryType: EntryType.chat,
      ownerId: myAddress,
      meta: {
        transactionHash: hash,
        index,
      } as SenseTransactionMeta,
    };

    // if no sync item(first message/initial)
    if (!syncItem) {
      const unreadCount = chat.transactions.filter(
        (t) => t.timestamp > chat.lastSendTimestamp
      ).length; // uread count on top of my last send message

      const newItem = {
        ...syncItemHeader,
        id: chat.userAddress,
        unreadCount,
        // if 'fast' then no shift update poiter till 'slow' reupdate
        timestampUpdate: shouldUpdateTimestamp ? transactionTimestamp : 0,
        timestampRead: chat.lastSendTimestamp,
        disabled: false,
      };

      // eslint-disable-next-line no-await-in-loop
      await throwIfAborted(db.putSyncStatus.bind(db), signal)(newItem);

      results.push({ ...newItem, meta: lastTransaction });
    } else {
      const { id, timestampRead, timestampUpdate, meta, unreadCount: prevUnreadCount } = syncItem;

      const lastTimestampRead = Math.max(timestampRead!, chat.lastSendTimestamp);
      const { timestampUpdateContent = 0, timestampUpdateChat = 0 } = meta;
      const timestampUnreadFrom = Math.max(chat.lastSendTimestamp, timestampUpdateChat);
      const unreadCount =
        prevUnreadCount + chat.transactions.filter((t) => t.timestamp > timestampUnreadFrom).length; // + new messages count

      if (timestampUpdate < transactionTimestamp) {
        // if message source is 'fast' then no update till 'slow' reupdate
        const newTimestampUpdateChat = shouldUpdateTimestamp
          ? transactionTimestamp
          : timestampUpdateChat;

        const syncStatusChanges = {
          ...syncItemHeader,
          id: id!,
          unreadCount,
          timestampRead: lastTimestampRead,
          // show max timestamp to use in sorting, in sense list
          // real timestamp shold be resynced with 'slow' data source by timestampUpdateChat
          timestampUpdate: Math.max(
            transactionTimestamp,
            timestampUpdateContent,
            newTimestampUpdateChat
          ),

          meta: {
            ...syncItemHeader.meta,
            timestampUpdateChat: newTimestampUpdateChat,
            timestampUpdateContent,
          },
        };

        // eslint-disable-next-line no-await-in-loop
        await throwIfAborted(db.updateSyncStatus.bind(db), signal)(syncStatusChanges);

        results.push({
          ...syncItem,
          ...syncStatusChanges,
          meta: lastTransaction,
        } as SenseListItem);
      }
    }
  }
  return results;
};
