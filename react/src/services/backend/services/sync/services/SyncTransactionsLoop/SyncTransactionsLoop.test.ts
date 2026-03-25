import { of } from 'rxjs';
import { CID_TWEET } from 'src/constants/app';
import DbApi, {
  mockGetSyncStatus,
  mockPutSyncStatus,
} from 'src/services/backend/services/dataSource/indexedDb/__mocks__/dbApiWrapperMock';
import { fetchCyberlinksIterable } from 'src/services/backend/services/indexer/cyberlinks';
import { EntryType } from 'src/services/CozoDb/types/entities';
import { CybIpfsNode } from 'src/services/ipfs/types';
import { createAsyncIterable } from 'src/utils/async/iterable';
import { dateToUtcNumber } from 'src/utils/date';
import { fetchTransactionsIterable } from '../../../indexer/transactions';
import { CYBER_LINK_TRANSACTION_TYPE } from '../../../indexer/types';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import SyncTransactionsLoop from './SyncTransactionsLoop';

jest.mock('src/services/backend/services/dataSource/blockchain/requests');
jest.mock('src/services/backend/services/dataSource/indexedDb/dbApiWrapper');
jest.mock('src/services/backend/channels/BroadcastChannelSender');
jest.mock('./services/chat');

describe('SyncTransactionsLoop', () => {
  let syncTransactionsLoop: SyncTransactionsLoop;
  let mockSyncQueue;
  const myAddress = 'user-address';
  beforeEach(() => {
    mockPutSyncStatus.mockClear();
    mockGetSyncStatus.mockClear();

    DbApi.mockClear();
    mockGetSyncStatus.mockResolvedValueOnce({
      ownerId: myAddress,
      id: myAddress,
      unreadCount: 0,
      timestampUpdate: 333,
      timestampRead: 222,
    });
    // mockFindSyncStatus.mockResolvedValueOnce([
    //   {
    //     id: myAddress,
    //     unreadCount: 0,
    //     timestampUpdate: 333,
    //     timestampRead: 222,
    //   },
    // ]);

    const db = new DbApi();
    const mockServiceDeps: ServiceDeps = {
      dbInstance$: of(db),
      ipfsInstance$: of({} as CybIpfsNode),
      params$: of({
        myAddress,
        followings: [],
        cyberIndexUrl: 'test-index-url',
      }),
      waitForParticleResolve: jest.fn(),
    };
    mockSyncQueue = new ParticlesResolverQueue(mockServiceDeps);

    syncTransactionsLoop = new SyncTransactionsLoop(mockServiceDeps, mockSyncQueue);
  });

  it('should call fetchTransactionsIterable and putSyncStatus correctly', (done) => {
    const particleTest = 'cid';

    const mockTransactionsBatched = [
      [
        {
          type: CYBER_LINK_TRANSACTION_TYPE,
          value: {
            links: [{ from: CID_TWEET, to: particleTest }],
          },
          transaction: {
            block: { timestamp: '2021-01-01' },
            transaction_hash: 'hash123',
          },
        },
      ],
    ];

    const mockCyberlinksDataBatched = [
      [
        { from: particleTest, to: 'mockTo2', timestamp: '2022-01-10' },
        { from: 'mockFrom1', to: particleTest, timestamp: '2022-01-01' },
      ],
    ];

    (fetchTransactionsIterable as jest.Mock).mockReturnValueOnce(
      createAsyncIterable(mockTransactionsBatched)
    );

    (fetchCyberlinksIterable as jest.Mock).mockReturnValueOnce(
      createAsyncIterable(mockCyberlinksDataBatched)
    );

    syncTransactionsLoop.start().loop$.subscribe({
      next: () => {
        expect(mockPutSyncStatus).toHaveBeenCalledWith([
          {
            id: particleTest,
            ownerId: 'user-address',
            entryType: EntryType.particle,
            timestampUpdate: dateToUtcNumber('2022-01-10'),
            timestampRead: dateToUtcNumber('2021-01-01'),
            unreadCount: 2,
            lastId: 'mockTo2',
            disabled: false,
            meta: { direction: 'from' },
          },
        ]);
        done();
      },
      error: (err) => done(err),
    });
  });
});
