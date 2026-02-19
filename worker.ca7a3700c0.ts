import { proxy } from 'comlink';
import { QueuePriority } from 'src/services/QueueManager/types';
import { BehaviorSubject, Subject } from 'rxjs';
import { exposeWorkerApi } from '../factoryMethods';
import { SyncService } from '../../services/sync/sync';
import BroadcastChannelSender from '../../channels/BroadcastChannelSender';
import { createIpfsApi } from './api/ipfsApi';
import { createMlApi } from './api/mlApi';
import { createRuneApi } from './api/runeApi';
// import { initRuneDeps } from 'src/services/scripting/wasmBindings';
const createBackgroundWorkerApi = ()=>{
    const broadcastApi = new BroadcastChannelSender();
    const dbInstance$ = new Subject();
    const injectDb = (db)=>dbInstance$.next(db);
    const params$ = new BehaviorSubject({
        myAddress: null
    });
    const { embeddingApi$ } = createMlApi(dbInstance$, broadcastApi);
    const { setInnerDeps, rune } = createRuneApi(embeddingApi$, dbInstance$, broadcastApi);
    const { ipfsQueue, ipfsInstance$, api: ipfsApi } = createIpfsApi(rune, broadcastApi);
    const waitForParticleResolve = function(cid) {
        let priority = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : QueuePriority.MEDIUM;
        return ipfsQueue.enqueueAndWait(cid, {
            postProcessing: false,
            priority
        });
    };
    const serviceDeps = {
        waitForParticleResolve,
        dbInstance$,
        ipfsInstance$,
        embeddingApi$,
        params$
    };
    // service to sync updates about cyberlinks, transactions, swarm etc.
    const syncService = new SyncService(serviceDeps);
    // INITIALIZATION
    setInnerDeps({
        ipfsApi
    });
    return {
        injectDb,
        isIpfsInitialized: ()=>!!ipfsInstance$.getValue(),
        // syncDrive,
        ipfsApi: proxy(ipfsApi),
        rune: proxy(rune),
        embeddingApi$,
        // ipfsInstance$,
        ipfsQueue: proxy(ipfsQueue),
        setRuneDeps: (deps)=>setInnerDeps(deps),
        // restartSync: (name: SyncEntryName) => syncService.restart(name),
        setParams: (params)=>params$.next({
                ...params$.value,
                ...params
            })
    };
};
const backgroundWorker = createBackgroundWorkerApi();
// Expose the API to the main thread as shared/regular worker
exposeWorkerApi(self, backgroundWorker);
