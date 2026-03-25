import { of } from 'rxjs';
import DbApi, {
  mockDeletePins,
  mockGetParticles,
  mockGetPins,
  mockPutPins,
  mockPutSyncQueue,
} from 'src/services/backend/services/dataSource/indexedDb/__mocks__/dbApiWrapperMock';
import { fetchPins } from 'src/services/backend/services/sync/services/SyncIpfsLoop/services';
import { CybIpfsNode } from 'src/services/ipfs/types';
import { QueuePriority } from 'src/services/QueueManager/types';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import SyncIpfsLoop from './SyncIpfsLoop';

jest.mock('src/services/backend/services/dataSource/blockchain/requests');
jest.mock('src/services/backend/services/dataSource/indexedDb/dbApiWrapper');
jest.mock('src/services/backend/channels/BroadcastChannelSender');
jest.mock('src/services/backend/services/dataSource/ipfs/ipfsSource');

describe('SyncIpfsLoop', () => {
  let syncIpfsLoop: SyncIpfsLoop;
  let mockSyncQueue;

  const myAddress = 'user-address';
  beforeEach(() => {
    mockDeletePins.mockClear();
    mockGetParticles.mockClear();
    mockGetPins.mockClear();
    mockPutPins.mockClear();
    // fetchPins.mockClear();

    DbApi.mockClear();

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

    syncIpfsLoop = new SyncIpfsLoop(mockServiceDeps, mockSyncQueue);
  });

  it('should process syncPins correctly', (done) => {
    (fetchPins as jest.Mock).mockResolvedValue([
      { cid: 'pin1', type: 'recursive' },
      { cid: 'pin2', type: 'recursive' },
      { cid: 'pin3', type: 'recursive' },
    ]);

    mockGetPins.mockResolvedValue({
      rows: [['pin1'], ['pin2'], ['dbpin1']],
    });

    mockGetParticles.mockResolvedValue({
      rows: [['pin1']],
    });

    syncIpfsLoop.start().loop$!.subscribe({
      next: () => {
        expect(mockPutSyncQueue).toHaveBeenCalledWith([
          {
            id: 'pin3',
            priority: QueuePriority.LOW,
          },
        ]);
        expect(mockDeletePins).toHaveBeenCalledWith(['dbpin1']);
        expect(mockPutPins).toHaveBeenCalledWith([{ cid: 'pin3', type: 1 }]);

        done();
      },
      error: (err) => done(err),
    });
  });
});
