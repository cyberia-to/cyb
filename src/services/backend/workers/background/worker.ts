import { proxy } from 'comlink';
import { BehaviorSubject, Subject } from 'rxjs';
import { QueuePriority } from 'src/services/QueueManager/types';
import { RuneInnerDeps } from 'src/services/scripting/runeDeps';
import { ParticleCid } from 'src/types/base';
import BroadcastChannelSender from '../../channels/BroadcastChannelSender';
import DbApi from '../../services/DbApi/DbApi';
import { SyncService } from '../../services/sync/sync';
import { SyncServiceParams } from '../../services/sync/types';
import { exposeWorkerApi } from '../factoryMethods';
import { createIpfsApi } from './api/ipfsApi';
import { createMlApi } from './api/mlApi';
import { createRuneApi } from './api/runeApi';

// import { initRuneDeps } from 'src/services/scripting/wasmBindings';

const createBackgroundWorkerApi = () => {
  const broadcastApi = new BroadcastChannelSender();

  const dbInstance$ = new Subject<DbApi>();

  const injectDb = (db: DbApi) => dbInstance$.next(db);

  const params$ = new BehaviorSubject<SyncServiceParams>({
    myAddress: null,
  });

  const { embeddingApi$ } = createMlApi(dbInstance$, broadcastApi);

  const { setInnerDeps, rune } = createRuneApi(embeddingApi$, dbInstance$, broadcastApi);

  const { ipfsQueue, ipfsInstance$, api: ipfsApi } = createIpfsApi(rune, broadcastApi);

  const waitForParticleResolve = (
    cid: ParticleCid,
    priority: QueuePriority = QueuePriority.MEDIUM
  ) => ipfsQueue.enqueueAndWait(cid, { postProcessing: false, priority });

  const serviceDeps = {
    waitForParticleResolve,
    dbInstance$,
    ipfsInstance$,
    embeddingApi$,
    params$,
  };

  // service to sync updates about cyberlinks, transactions, swarm etc.
  const _syncService = new SyncService(serviceDeps);

  // INITIALIZATION
  setInnerDeps({ ipfsApi });

  return {
    injectDb,
    isIpfsInitialized: () => !!ipfsInstance$.getValue(),
    // syncDrive,
    ipfsApi: proxy(ipfsApi),
    rune: proxy(rune),
    embeddingApi$,
    // ipfsInstance$,
    ipfsQueue: proxy(ipfsQueue),
    setRuneDeps: (deps: Partial<Omit<RuneInnerDeps, 'embeddingApi' | 'dbApi'>>) =>
      setInnerDeps(deps),
    // restartSync: (name: SyncEntryName) => syncService.restart(name),
    setParams: (params: Partial<SyncServiceParams>) =>
      params$.next({ ...params$.value, ...params }),
  };
};

const backgroundWorker = createBackgroundWorkerApi();

export type BackgroundWorker = typeof backgroundWorker;

// Expose the API to the main thread as shared/regular worker
exposeWorkerApi(self, backgroundWorker);
