import { of } from 'rxjs';
import { fetchCyberlinksIterable } from 'src/services/backend/services/indexer/cyberlinks';
import { CybIpfsNode } from 'src/services/ipfs/types';
import { createAsyncIterable } from 'src/utils/async/iterable';
import { numberToUtcDate } from 'src/utils/date';
import DbApi, {
  mockFindSyncStatus,
  mockPutSyncStatus,
} from '../../../dataSource/indexedDb/__mocks__/dbApiWrapperMock';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import SyncParticlesLoop from './SyncParticlesLoop';

jest.mock('src/services/backend/services/dataSource/blockchain/requests');
jest.mock('src/services/backend/services/dataSource/indexedDb/dbApiWrapper');
jest.mock('src/services/backend/channels/BroadcastChannelSender');

describe('SyncParticlesLoop', () => {
  let syncParticlesLoop: SyncParticlesLoop;
  let mockSyncQueue;
  beforeEach(() => {
    mockPutSyncStatus.mockClear();
    DbApi.mockClear();

    const db = new DbApi();
    const mockServiceDeps: ServiceDeps = {
      dbInstance$: of(db),
      ipfsInstance$: of({} as CybIpfsNode),
      params$: of({
        myAddress: 'r',
        followings: [],
        cyberIndexUrl: 'test-index-url',
      }),
      waitForParticleResolve: jest.fn(),
    };
    mockSyncQueue = new ParticlesResolverQueue(mockServiceDeps);

    syncParticlesLoop = new SyncParticlesLoop(mockServiceDeps, mockSyncQueue);
  });

  it('should call updateSyncState and putSyncStatus correctly', (done) => {
    (fetchCyberlinksIterable as jest.Mock).mockReturnValueOnce(
      createAsyncIterable([[{ from: 'cid', to: 'cid1', timestamp: numberToUtcDate(333) }]])
    );

    mockFindSyncStatus.mockResolvedValueOnce([
      {
        id: 'cid',
        unreadCount: 0,
        timestampUpdate: 123,
        timestampRead: 123,
      },
    ]);

    const resultSyncStatus = {
      id: 'cid',
      timestampUpdate: 333,
      timestampRead: 123,
      unreadCount: 1,
      lastId: 'cid1',
      meta: { direction: 'from' },
    };

    syncParticlesLoop.start().loop$!.subscribe({
      next: () => {
        expect(mockPutSyncStatus).toHaveBeenCalledWith([resultSyncStatus]);
        done();
      },
      error: (err) => done(err),
    });
  });
});
