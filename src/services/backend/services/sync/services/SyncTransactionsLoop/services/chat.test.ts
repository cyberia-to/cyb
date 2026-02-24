import { EntryType } from 'src/services/CozoDb/types/entities';
import { dateToUtcNumber } from 'src/utils/date';
import DbApi, {
  mockFindSyncStatus,
  mockGetSyncStatus,
  mockGetTransactions,
  mockPutSyncStatus,
  mockUpdateSyncStatus,
} from '../../../../dataSource/indexedDb/__mocks__/dbApiWrapperMock';
import {
  MSG_MULTI_SEND_TRANSACTION_TYPE,
  MSG_SEND_TRANSACTION_TYPE,
} from '../../../../indexer/types';

import { syncMyChats } from './chat';

jest.mock('src/services/backend/channels/BroadcastChannelSender');

describe('SyncMyChatsLoop', () => {
  const myAddress = 'my-address';
  const db = new DbApi();

  beforeEach(() => {
    mockPutSyncStatus.mockClear();
    mockGetSyncStatus.mockClear();
    mockFindSyncStatus.mockClear();
    mockUpdateSyncStatus.mockClear();
  });

  it('should sync my chats', async () => {
    const mockTransactions = [
      {
        type: MSG_SEND_TRANSACTION_TYPE,
        value: {
          amount: [{ denom: 'ATOM', amount: '100' }],
          from_address: myAddress,
          to_address: 'receiver1',
        },
        timestamp: dateToUtcNumber('2022-01-01'),
        hash: 'hash000',
      },
      {
        type: MSG_SEND_TRANSACTION_TYPE,
        value: {
          amount: [{ denom: 'ATOM', amount: '10' }],
          from_address: myAddress,
          to_address: 'receiver1',
        },
        timestamp: dateToUtcNumber('2022-01-10'),
        hash: 'hash321',
        memo: 'hello test',
      },
      {
        type: MSG_MULTI_SEND_TRANSACTION_TYPE,
        value: {
          inputs: [{ address: 'sender2', coins: [{ amount: '50', denom: 'ATOM' }] }],
          outputs: [{ address: myAddress, coins: [{ amount: '50', denom: 'ATOM' }] }],
        },
        timestamp: dateToUtcNumber('2022-01-02'),
        hash: 'hash456',
      },
    ];

    mockFindSyncStatus.mockResolvedValueOnce([
      {
        id: 'sender2',
        timestampUpdate: dateToUtcNumber('2022-01-01'),
        unreadCount: 1,
        timestampRead: 0,
        disabled: false,
        lastId: '',
        meta: {},
      },
    ]);

    mockGetTransactions.mockResolvedValueOnce(mockTransactions);

    await syncMyChats(db, myAddress);
    expect(mockFindSyncStatus).toHaveBeenCalledWith({
      entryType: EntryType.chat,
      ownerId: 'my-address',
    });

    expect(mockPutSyncStatus).toHaveBeenCalledWith({
      entryType: EntryType.chat,
      id: 'receiver1',
      ownerId: 'my-address',
      timestampUpdate: dateToUtcNumber('2022-01-10'),
      unreadCount: 2,
      timestampRead: 0,
      disabled: false,
      lastId: 'hash321',
      meta: {
        amount: [{ denom: 'ATOM', amount: '10' }],
        memo: 'hello test',
        direction: 'to',
      },
    });

    expect(mockUpdateSyncStatus).toHaveBeenCalledWith({
      id: 'sender2',
      timestampUpdate: dateToUtcNumber('2022-01-02'),
      unreadCount: 1,
      ownerId: 'my-address',
      meta: {
        amount: [{ denom: 'ATOM', amount: '50' }],
        direction: 'from',
        memo: undefined,
      },
    });
  });
});
