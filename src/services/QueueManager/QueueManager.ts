import * as R from 'ramda';
import {
  BehaviorSubject,
  catchError,
  combineLatest,
  debounceTime,
  distinctUntilChanged,
  EMPTY,
  filter,
  interval,
  map,
  merge,
  mergeMap,
  Observable,
  of,
  share,
  throwError,
  timeout,
  withLatestFrom,
} from 'rxjs';

import { CybIpfsNode, IpfsContentSource } from 'src/services/ipfs/types';
import { fetchIpfsContent } from 'src/services/ipfs/utils/utils-ipfs';
import { ParticleCid } from 'src/types/base';

import { promiseToObservable } from '../../utils/rxjs/helpers';
import { enqueueParticleSave } from '../backend/channels/BackendQueueChannel/backendQueueSenders';
import BroadcastChannelSender from '../backend/channels/BroadcastChannelSender';
import { CustomHeaders, XCybSourceValues } from './constants';
import { QueueItemTimeoutError } from './QueueItemTimeoutError';
import { QueueStrategy } from './QueueStrategy';
import type {
  QueueItem,
  QueueItemAsyncResult,
  QueueItemCallback,
  QueueItemOptions,
  QueueItemResult,
  QueueSource,
  QueueStats,
} from './types';

const QUEUE_DEBOUNCE_MS = 33;
const CONNECTION_KEEPER_RETRY_MS = 15000;

function getQueueItemTotalPriority(item: QueueItem): number {
  return (item.priority || 0) + (item.viewPortPriority || 0);
}

const _debugCid = (cid: ParticleCid, prefix: string, ...args) => {
  console.log(`>>> ${prefix}: ${cid}`, ...args);
};

const strategies = {
  external: new QueueStrategy(
    {
      db: { timeout: 5000, maxConcurrentExecutions: 999 },
      node: { timeout: 60 * 1000, maxConcurrentExecutions: 30 },
      gateway: { timeout: 10000, maxConcurrentExecutions: 11 },
    },
    ['db', 'node', 'gateway']
  ),
  embedded: new QueueStrategy(
    {
      db: { timeout: 5000, maxConcurrentExecutions: 999 },
      node: { timeout: 60 * 1000, maxConcurrentExecutions: 30 },
      gateway: { timeout: 21000, maxConcurrentExecutions: 11 },
    },
    ['db', 'gateway', 'node']
  ),
  helia: new QueueStrategy(
    {
      db: { timeout: 5000, maxConcurrentExecutions: 999 },
      node: { timeout: 15 * 1000, maxConcurrentExecutions: 50 },
      gateway: { timeout: 15000, maxConcurrentExecutions: 30 },
    },
    ['db', 'gateway', 'node']
  ),
};

type QueueMap = Map<ParticleCid, QueueItem>;

class QueueManager {
  private queue$ = new BehaviorSubject<QueueMap>(new Map());

  private node: CybIpfsNode | undefined = undefined;

  private strategy: QueueStrategy;

  private queueDebounceMs: number;

  private channel = new BroadcastChannelSender();

  private executing: Record<QueueSource, Set<ParticleCid>> = {
    db: new Set(),
    node: new Set(),
    gateway: new Set(),
  };

  private switchStrategy(strategy: QueueStrategy): void {
    this.strategy = strategy;
  }

  public async setNode(node: CybIpfsNode, customStrategy?: QueueStrategy) {
    console.log(`* switch node from ${this.node?.nodeType || '<none>'} to ${node.nodeType}`);
    this.node = node;
    this.switchStrategy(customStrategy || strategies[node.nodeType]);
  }

  private getItemBySourceAndPriority(queue: QueueMap) {
    const pendingItems = [...queue.values()].filter((i) => i.status === 'pending');

    const pendingBySource = R.groupBy((i) => i.source, pendingItems);

    const itemsToExecute: QueueItem[] = [];
    // eslint-disable-next-line no-loop-func, no-restricted-syntax
    for (const [queueSource, items] of Object.entries(pendingBySource)) {
      const settings = this.strategy.settings[queueSource as IpfsContentSource];

      const executeCount =
        settings.maxConcurrentExecutions - this.executing[queueSource as IpfsContentSource].size;
      const itemsByPriority = items
        .sort((a, b) => getQueueItemTotalPriority(b) - getQueueItemTotalPriority(a))
        .slice(0, executeCount);

      itemsToExecute.push(...itemsByPriority);
    }

    return itemsToExecute;
  }

  private postSummary() {
    const summary = `(total: ${this.queue$.value.size} |  db - ${this.executing.db.size} node - ${this.executing.node.size} gateway - ${this.executing.gateway.size})`;
    this.channel.postServiceStatus('ipfs', 'started', summary);
  }

  private fetchData$(item: QueueItem) {
    const { cid, source, callbacks, controller } = item;
    console.log(`🔄 fetchData: ${cid.slice(0, 12)}... source=${source}`);
    // const abortController = controller || new AbortController();
    const settings = this.strategy.settings[source];
    this.executing[source].add(cid);
    this.postSummary();
    const queueItem = this.queue$.value.get(cid);
    // Mutate item without next
    this.queue$.value.set(cid, {
      ...queueItem,
      status: 'executing',
      executionTime: Date.now(),
      controller: new AbortController(),
    } as QueueItem);
    // debugCid(cid, 'fetchData', cid, source);
    callbacks.map((callback) => callback(cid, 'executing', source));

    return promiseToObservable(async () => {
      return fetchIpfsContent(cid, source, {
        controller,
        node: this.node,
        headers: {
          [CustomHeaders.XCybSource]: XCybSourceValues.sharedWorker,
        },
      }).then((content) => {
        // put saveto db msg into bus
        if (content && source !== 'db') {
          enqueueParticleSave(content);
        }

        return content;
      });
    }).pipe(
      timeout({
        each: settings.timeout,
        with: () =>
          throwError(() => {
            controller?.abort('timeout');

            return new QueueItemTimeoutError(settings.timeout);
          }),
      }),
      map((result): QueueItemResult => {
        return {
          item,
          status: result?.result ? 'completed' : 'error',
          source,
          result,
        };
      }),
      catchError((error): Observable<QueueItemResult> => {
        // debugCid(cid, 'fetchData - fetchIpfsContent catchErr', error);
        if (error instanceof QueueItemTimeoutError) {
          return of({
            item,
            status: 'timeout',
            source,
          });
        }

        if (error?.name === 'AbortError') {
          return of({ item, status: 'cancelled', source });
        }
        return of({ item, status: 'error', source });
      })
    );
  }

  /**
   * Mutate queue item, and return new queue
   * @param cid
   * @param changes
   * @returns
   */
  private mutateQueueItem(cid: string, changes: Partial<QueueItem>) {
    const queue = this.queue$.value;
    const item = queue.get(cid);
    if (item) {
      queue.set(cid, { ...item, ...changes });
    }

    return this.queue$.next(queue);
  }

  private removeAndNext(cid: string): void {
    const queue = this.queue$.value;
    queue.delete(cid);
    this.queue$.next(queue);
  }

  // reset status and switch to next source
  private switchSourceAndNext(item: QueueItem, nextSource: QueueSource): void {
    item.callbacks.map((callback) => callback(item.cid, 'pending', nextSource));

    this.mutateQueueItem(item.cid, { status: 'pending', source: nextSource });
  }

  private cancelDeprioritizedItems(queue: QueueMap): QueueMap {
    (['node', 'gateway'] as IpfsContentSource[]).forEach((source) => {
      Array.from(this.executing[source]).forEach((cid) => {
        const item = queue.get(cid);
        if (item && getQueueItemTotalPriority(item) < 0 && item.controller) {
          // abort request and move to pending
          item.controller.abort('cancelled');
          item.callbacks.map((callback) => callback(item.cid, 'pending', item.source));

          queue.set(cid, { ...item, status: 'pending' });
          // console.log('-----cancel item', item, queue);

          this.executing[source].delete(cid);
        }
      });
    });

    return queue;
  }

  private releaseExecution(cid: string) {
    // eslint-disable-next-line no-restricted-syntax
    Object.keys(this.executing).forEach((key) =>
      this.executing[key as IpfsContentSource].delete(cid)
    );
  }

  constructor(
    ipfsInstance$: Observable<CybIpfsNode | undefined>,
    {
      strategy,
      queueDebounceMs,
    }: {
      strategy?: QueueStrategy;
      queueDebounceMs?: number;
    }
  ) {
    ipfsInstance$.subscribe((node) => {
      if (node) {
        this.setNode(node);
      }
    });

    this.strategy = strategy || strategies.external;
    this.queueDebounceMs = queueDebounceMs || QUEUE_DEBOUNCE_MS;

    // Little hack to handle keep-alive connection to swarm cyber node
    // Fix some lag with node peers(when it shown swarm node in peers but not  connected anymore)
    interval(CONNECTION_KEEPER_RETRY_MS)
      .pipe(
        filter(
          () => !!this.node && !![...this.queue$.value.values()].find((i) => i.source === 'node')
        )
      )
      .subscribe(() => {
        // console.log(
        //   '-----reconnect cnt',
        //   this.queue$.value.size,
        //   this.queue$.value
        // );
        this.node!.reconnectToSwarm(true);
      });

    const isInitialized$ = combineLatest([ipfsInstance$]).pipe(
      map(([ipfsInstance]) => !!ipfsInstance && ipfsInstance?.isStarted),
      distinctUntilChanged(),
      share()
    );

    isInitialized$.subscribe((isInitialized) => {
      isInitialized && console.log('🚦 ipfs queue initialized');
      this.node?.reconnectToSwarm(true);
    });

    this.queue$
      .pipe(
        debounceTime(this.queueDebounceMs),
        map((queue) => this.cancelDeprioritizedItems(queue)),
        mergeMap((queue) => {
          const workItems = this.getItemBySourceAndPriority(queue);
          // console.log('---workItems', workItems);
          if (workItems.length > 0) {
            // wake up connnection to swarm cyber node
            this.node?.reconnectToSwarm(false);

            return merge(...workItems.map((item) => this.fetchData$(item)));
          }
          return EMPTY;
        })
      )
      .subscribe(({ item, status, source, result }) => {
        const { cid } = item;
        console.log(`📦 result: ${cid.slice(0, 12)}... source=${source} status=${status} hasResult=${!!result?.result}`);
        const callbacks = this.queue$.value.get(cid)?.callbacks || [];

        callbacks.map((callback) => callback(cid, status, source, result));

        // HACK to use with GracePeriod for reconnection
        if (source === 'node') {
          this.lastNodeCallTime = Date.now();
        }

        this.executing[source].delete(cid);

        // success execution -> next
        if (status === 'completed' || status === 'cancelled') {
          // debugCid(cid, '------done', item, status, source, result);
          this.removeAndNext(cid);
        } else {
          // debugCid(cid, '------error', item, status, source, result);

          // Retry -> (next sources) or -> next
          const nextSource = this.strategy.getNextSource(source);

          if (nextSource) {
            this.switchSourceAndNext(item, nextSource);
          } else {
            this.removeAndNext(cid);
            // notify thatn nothing found from all sources
            callbacks.map((callback) => callback(cid, 'not_found', source, result));
          }
        }

        this.postSummary();
      });
  }

  public enqueue(cid: string, callback: QueueItemCallback, options: QueueItemOptions = {}): void {
    const queue = this.queue$.value;
    const existingItem = queue.get(cid);
    console.log(`📥 enqueue: ${cid.slice(0, 12)}... source=${this.strategy.order[0]} queueSize=${queue.size}`);

    // In case if item already in queue,
    // just attach one more callback to quieued item
    if (existingItem) {
      this.mutateQueueItem(cid, {
        callbacks: [...existingItem.callbacks, callback],
      });
    } else {
      const source = options.initialSource || this.strategy.order[0];
      const item: QueueItem = {
        cid,
        callbacks: [callback],
        source, // initial method to fetch
        status: 'pending',
        postProcessing: true, // by default rune-post-processing enabled
        ...options,
      };

      callback(cid, 'pending', source);

      queue.set(cid, item);
      this.queue$.next(queue);
    }
  }

  public enqueueAndWait(
    cid: string,
    options: QueueItemOptions = {}
  ): Promise<QueueItemAsyncResult> {
    return new Promise((resolve) => {
      const callback = ((_cid, status, source, result) => {
        if (status === 'completed' || status === 'not_found') {
          resolve({ status, source, result });
        }
      }) as QueueItemCallback;

      this.enqueue(cid, callback, options);
    });
  }

  public updateViewPortPriority(cid: string, viewPortPriority: number) {
    this.mutateQueueItem(cid, { viewPortPriority });
  }

  public cancel(cid: string): void {
    const queue = this.queue$.value;
    const item = queue.get(cid);
    // console.log('-----cancel item', item, item?.controller);
    if (item) {
      // If item has no abortController we can just remove it,
      // otherwise abort&keep-to-finalize
      if (!item.controller) {
        this.removeAndNext(cid);
      } else {
        item.controller.abort('cancelled');
      }
    }
  }

  public cancelByParent(parent: string): void {
    const queue = this.queue$.value;

    queue.forEach((item, cid) => {
      if (item.parent === parent) {
        this.releaseExecution(cid);
        item.controller?.abort('cancelled');
        queue.delete(cid);
      }
    });

    this.queue$.next(queue);
  }

  public clear(): void {
    const queue = this.queue$.value;

    queue.forEach((item, cid) => {
      this.releaseExecution(cid);
      item.controller?.abort('cancelled');
      queue.delete(cid);
    });

    this.queue$.next(new Map());
  }

  public getQueueMap(): QueueMap {
    return this.queue$.value;
  }

  public getQueueList(): QueueItem[] {
    return Array.from(this.queue$.value.values());
  }

  public getStats(): QueueStats[] {
    const fn = R.pipe(
      R.countBy<QueueItem>(R.prop('status')),
      R.toPairs,
      R.map(R.zipObj(['status', 'count']))
    );

    return fn(this.getQueueList()) as QueueStats[];
  }
}

export default QueueManager;
