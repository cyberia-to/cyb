import { combineLatest, defer, from, map, Observable } from 'rxjs';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { broadcastStatus } from 'src/services/backend/channels/broadcastStatus';
import { mapPinToEntity } from 'src/services/CozoDb/mapping';
import { SyncQueueJobType } from 'src/services/CozoDb/types/entities';
import { CybIpfsNode } from 'src/services/ipfs/types';
import { QueuePriority } from 'src/services/QueueManager/types';
import DbApi from '../../../DbApi/DbApi';
import { IPFS_SYNC_INTERVAL } from '../consts';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import { createLoopObservable } from '../utils/rxjs/loop';
import { fetchPins } from './services';

class SyncIpfsLoop {
  private isInitialized$: Observable<boolean>;

  private db: DbApi | undefined;

  private ipfsNode: CybIpfsNode | undefined;

  private particlesResolver: ParticlesResolverQueue | undefined;

  private statusApi = broadcastStatus('pin', new BroadcastChannelSender());

  private _loop$: Observable<any> | undefined;

  public get loop$(): Observable<any> | undefined {
    return this._loop$;
  }

  constructor(deps: ServiceDeps, particlesResolver: ParticlesResolverQueue) {
    deps.dbInstance$.subscribe((db) => {
      this.db = db;
    });

    deps.ipfsInstance$.subscribe((ipfsInstance) => {
      this.ipfsNode = ipfsInstance;
    });

    this.particlesResolver = particlesResolver;

    // this.isInitialized$ = isInitialized$;

    this.isInitialized$ = combineLatest([
      deps.dbInstance$,
      deps.ipfsInstance$,
      particlesResolver.isInitialized$,
    ]).pipe(
      map(
        ([dbInstance, ipfsInstance, syncQueueInitialized]) =>
          !!ipfsInstance && !!dbInstance && !!syncQueueInitialized
      )
    );
  }

  start() {
    const { loop$ } = createLoopObservable(
      this.isInitialized$,
      defer(() => from(this.syncPins())),
      { intervalMs: IPFS_SYNC_INTERVAL }
    );

    this._loop$ = loop$;
    this._loop$.subscribe({
      next: (_result) => this.statusApi.sendStatus('active'),
      error: (err) => this.statusApi.sendStatus('error', err.toString()),
    });

    return this;
  }

  private async syncPins() {
    try {
      this.statusApi.sendStatus('in-progress');

      const pinsResult = await fetchPins(this.ipfsNode!);
      // console.log('---syncPins pinsResult', pinsResult);
      const dbPins = (await this.db!.getPins()).rows.map((row) => row[0] as string);

      const pinsResultSet = new Set(pinsResult.map((pin) => pin.cid.toString()));
      const dbPinsSet = new Set(dbPins);

      // Find and exclude overlapping pins
      const pinsToRemove = dbPins.filter((pin) => !pinsResultSet.has(pin));
      const pinsToAdd = pinsResult.filter((pin) => !dbPinsSet.has(pin.cid.toString()));

      if (pinsToRemove.length) {
        await this.db!.deletePins(pinsToRemove);
      }

      const particlesExist = new Set(
        (await this.db!.getParticlesRaw(['cid'])).rows.map((row) => row[0] as string)
      );

      const cidsToAdd = pinsToAdd.map((pin) => pin.cid.toString());

      const particlesToAdd = cidsToAdd.filter((cid) => !particlesExist.has(cid));

      if (particlesToAdd.length > 0) {
        await this.particlesResolver!.enqueue(
          particlesToAdd.map((cid) => ({
            id: cid,
            jobType: SyncQueueJobType.particle,
            priority: QueuePriority.LOW,
          }))
        );
      }

      if (pinsToAdd.length > 0) {
        await this.db!.putPins(pinsToAdd.map(mapPinToEntity));
      }
    } catch (e) {
      console.log('---syncPins error', e);
      throw e;
    }
  }
}

export default SyncIpfsLoop;
