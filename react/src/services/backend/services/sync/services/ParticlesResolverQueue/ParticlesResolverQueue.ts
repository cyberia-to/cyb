import {
  BehaviorSubject,
  combineLatest,
  EMPTY,
  filter,
  first,
  map,
  mergeMap,
  Observable,
  share,
  tap,
} from 'rxjs';
import { PATTERN_COSMOS, PATTERN_CYBER } from 'src/constants/patterns';
import { enqueueParticleEmbeddingMaybe } from 'src/services/backend/channels/BackendQueueChannel/backendQueueSenders';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { broadcastStatus } from 'src/services/backend/channels/broadcastStatus';
import { EmbeddingApi } from 'src/services/backend/workers/background/api/mlApi';
import { SyncQueueJobType, SyncQueueStatus } from 'src/services/CozoDb/types/entities';
import { IPFSContent } from 'src/services/ipfs/types';
import { QueuePriority } from 'src/services/QueueManager/types';
import { Option } from 'src/types';
import { ParticleCid } from 'src/types/base';
import { asyncIterableBatchProcessor } from 'src/utils/async/iterable';
import DbApi from '../../../DbApi/DbApi';
import { FetchIpfsFunc } from '../../types';
import { MAX_DATABASE_PUT_SIZE } from '../consts';
import { ServiceDeps } from '../types';
import { SyncQueueItem } from './types';

const QUEUE_BATCH_SIZE = 100;

const getContentToEmbed = async (content: IPFSContent) => {
  const contentType = content?.meta?.contentType || '';

  // create embedding for allowed content
  if (contentType === 'text') {
    return [contentType, content.textPreview];
  }

  return [contentType, undefined];
};

export const getTextContentIfShouldEmbed = async (content: IPFSContent) => {
  const [contentType, data] = await getContentToEmbed(content);

  let shouldEmbed = contentType === 'text' && !!data;

  shouldEmbed = shouldEmbed && (!data!.match(PATTERN_COSMOS) || !data!.match(PATTERN_CYBER));

  return shouldEmbed ? data : undefined;
};

class ParticlesResolverQueue {
  public isInitialized$: Observable<boolean>;

  private db: Option<DbApi>;

  private embeddingApi: Option<EmbeddingApi>;

  private get canEmbed() {
    return !!this.embeddingApi;
  }

  private waitForParticleResolve: FetchIpfsFunc;

  private statusApi = broadcastStatus('resolver', new BroadcastChannelSender());

  private _syncQueue$ = new BehaviorSubject<Map<ParticleCid, SyncQueueItem>>(new Map());

  public get queue(): Map<ParticleCid, SyncQueueItem> {
    return this._syncQueue$.getValue();
  }

  private _loop$: Observable<any> | undefined;

  public get loop$(): Observable<any> | undefined {
    return this._loop$;
  }

  constructor(deps: ServiceDeps) {
    if (!deps.waitForParticleResolve) {
      throw new Error('waitForParticleResolve is not defined');
    }

    this.waitForParticleResolve = deps.waitForParticleResolve;

    deps.embeddingApi$.subscribe((embeddingApi) => {
      this.embeddingApi = embeddingApi;
      // if embedding function is provided, retriger the queue
      if (this.queue.size > 0) {
        this._syncQueue$.next(this.queue);
      }
    });

    deps.dbInstance$
      .pipe(
        first((value) => value !== undefined) // Automatically unsubscribes after the first valid value
      )
      .subscribe(async (db) => {
        this.db = db;
        await this.loadSyncQueue();
      });

    this.isInitialized$ = combineLatest([deps.dbInstance$, deps.ipfsInstance$]).pipe(
      map(([dbInstance, ipfsInstance]) => !!ipfsInstance && !!dbInstance)
    );
  }

  private async resolveIpfsParticle(id: ParticleCid, priority: QueuePriority) {
    return this.waitForParticleResolve(id, priority)
      .then(async ({ status, result }) => {
        const isResolved = status !== 'not_found';
        if (!isResolved || !result) {
          return false;
        }

        await enqueueParticleEmbeddingMaybe(result);
        return true;
      })
      .catch(() => false);
  }

  private async saveEmbedding(cid: ParticleCid, text: string) {
    try {
      const hasItem = await this.db!.existEmbedding(cid);

      if (!hasItem) {
        const vec = await this.embeddingApi!.createEmbedding(text);

        const _result = await this.db!.putEmbedding(cid, vec);
      }

      return true;
    } catch (err) {
      console.error(`saveEmbedding error: ${cid} - ${text} `, err.toString());
      return false;
    }
  }

  private async processSyncQueue(pendingItems: SyncQueueItem[]) {
    // console.log('------processSyncQueue ', pendingItems);

    const batchSize = pendingItems.length;

    this.statusApi.sendStatus(
      'in-progress',
      `processing batch ${batchSize}/${batchSize} batch. ${this.queue.size} pending...`
    );

    let i = batchSize;
    await Promise.all(
      pendingItems.map(async (item) => {
        const { id, jobType, data } = item;

        let jobPromise = Promise.resolve(false);

        if (jobType === SyncQueueJobType.embedding && data) {
          jobPromise = this.saveEmbedding(id, data as string);
        } else if (jobType === SyncQueueJobType.particle) {
          jobPromise = this.resolveIpfsParticle(id, QueuePriority.MEDIUM);
        }

        // eslint-disable-next-line no-await-in-loop
        return jobPromise.then(async (result) => {
          if (result) {
            await this.db!.removeSyncQueue({ id, jobType });
          } else {
            await this.db!.updateSyncQueue({
              id,
              jobType,
              status: SyncQueueStatus.error,
            });
          }

          const queue = this._syncQueue$.value;
          queue.delete(id);
          i--;
          this._syncQueue$.next(queue);

          this.statusApi.sendStatus(
            'in-progress',
            `processing batch ${batchSize - i}/${batchSize} batch. ${this.queue.size} pending...`
          );
        });
      })
    );
  }

  start() {
    const source$ = this.isInitialized$.pipe(
      tap((q) => console.log(`sync queue isInitialized - ${q}`)),
      filter((isInitialized) => isInitialized === true),
      mergeMap(() => this._syncQueue$), // Merge the queue$ stream here.
      // tap((q) => console.log(`sync queue - ${q.size}`)),
      filter((q) => q.size > 0),
      mergeMap((queue) => {
        const list = [...queue.values()];

        const executingCount = list.filter((i) => i.status === SyncQueueStatus.executing).length;

        const batchSize = QUEUE_BATCH_SIZE - executingCount;

        const jobTypeFilter = (i: SyncQueueItem) =>
          i.jobType === SyncQueueJobType.particle ||
          (i.jobType === SyncQueueJobType.embedding && this.canEmbed);

        if (batchSize > 0) {
          const pendingItems = list
            .filter((i) => i.status === SyncQueueStatus.pending && jobTypeFilter(i))
            .sort((a, b) => {
              return a.priority - b.priority;
            })
            .slice(0, batchSize);

          if (pendingItems.length > 0) {
            pendingItems.forEach((i) => {
              queue.set(i.id, {
                ...i,
                status: SyncQueueStatus.executing,
              });
            });

            this._syncQueue$.next(queue);

            this.statusApi.sendStatus('in-progress', `starting...`);
            return this.processSyncQueue(pendingItems);
          }
        }

        return EMPTY;
      })
    );

    this._loop$ = source$.pipe(share());

    this._loop$.subscribe({
      next: (_result) => {
        this.statusApi.sendStatus('active');
      },
      error: (err) => this.statusApi.sendStatus('error', err.toString()),
    });

    return this;
  }

  public async enqueueBatch(
    cids: ParticleCid[],
    jobType: SyncQueueJobType,
    priority: QueuePriority
  ) {
    return asyncIterableBatchProcessor(
      cids,
      (cids) =>
        this.enqueue(
          cids.map((cid) => ({
            id: cid /* from is tweet */,
            priority,
            jobType,
          }))
        ),
      MAX_DATABASE_PUT_SIZE
    );
  }

  public async enqueue(items: SyncQueueItem[]) {
    if (items.length === 0) {
      return;
    }

    const _result = await this.db!.putSyncQueue(items);

    const queue = this._syncQueue$.value;

    items.forEach((item) => queue.set(item.id, { ...item, status: SyncQueueStatus.pending }));
    this._syncQueue$.next(queue);
  }

  private async loadSyncQueue() {
    const queue = await this.db!.getSyncQueue({
      statuses: [SyncQueueStatus.pending],
    }).then((items) => new Map(items.map((item) => [item.id, item])));

    this._syncQueue$.next(new Map([...queue, ...this.queue]));
  }
}

export default ParticlesResolverQueue;
