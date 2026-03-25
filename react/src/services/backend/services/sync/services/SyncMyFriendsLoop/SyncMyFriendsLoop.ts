/* eslint-disable camelcase */
import { BehaviorSubject, combineLatest, distinctUntilChanged, map } from 'rxjs';
import { SenseItemLinkMeta } from 'src/services/backend/types/sense';
import { SyncEntryName } from 'src/services/backend/types/services';
import { mapLinkFromIndexerToDto } from 'src/services/CozoDb/mapping';
import { EntryType, SyncQueueJobType } from 'src/services/CozoDb/types/entities';
import { QueuePriority } from 'src/services/QueueManager/types';
import { NeuronAddress } from 'src/types/base';
import { throwIfAborted } from 'src/utils/async/promise';
import { entityToDto } from 'src/utils/dto';
import { isAbortException } from 'src/utils/exceptions/helpers';
import { CYBERLINKS_BATCH_LIMIT } from '../../../indexer/consts';

import { fetchCyberlinksByNerounIterable } from '../../../indexer/cyberlinks';
import { SyncServiceParams } from '../../types';
import { getLastReadInfo } from '../../utils';
import BaseSyncLoop from '../BaseSyncLoop/BaseSyncLoop';
import { SENSE_FRIEND_PARTICLES } from '../consts';

import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';

class SyncMyFriendsLoop extends BaseSyncLoop {
  protected followings: NeuronAddress[] = [];

  constructor(
    name: SyncEntryName,
    intervalMs: number,
    deps: ServiceDeps,
    particlesResolver: ParticlesResolverQueue,
    { warmupMs }: { warmupMs: number } = { warmupMs: 0 }
  ) {
    if (!deps.followings$) {
      throw new Error('followings$ is required');
    }

    super(name, intervalMs, deps, particlesResolver, {
      warmupMs,
    });
  }

  protected createIsInitializedObserver(deps: ServiceDeps) {
    const followingsInitialized$ = new BehaviorSubject<boolean>(false);
    deps.params$
      ?.pipe(
        map((params) => params.myAddress),
        distinctUntilChanged()
      )
      .subscribe(() => {
        followingsInitialized$.next(false);
      });

    deps.followings$!.subscribe((followings) => {
      this.followings = followings;
      followingsInitialized$.next(true);

      this.restart();
    });

    const isInitialized$ = combineLatest([
      deps.dbInstance$,
      deps.params$!,
      this.particlesResolver!.isInitialized$,
      followingsInitialized$!,
    ]).pipe(
      map(
        ([dbInstance, params, syncQueueInitialized, followingsInitialized]) =>
          !!dbInstance && !!params.myAddress && !!syncQueueInitialized && followingsInitialized
      )
    );

    return isInitialized$;
  }

  protected async sync(params: SyncServiceParams) {
    const { signal } = this.abortController;

    this.statusApi.sendStatus('in-progress', 'preparing...');
    const { myAddress } = params;

    const { followings } = this;

    this.statusApi.sendStatus('estimating');

    this.cyblogCh.info(`>>> syncMyFriends ${myAddress} count ${followings.length}`, {
      unit: 'friends-sync',
      data: followings,
    });

    this.progressTracker.start(followings.length);
    this.statusApi.sendStatus('in-progress', `sync...`, this.progressTracker.progress);

    // eslint-disable-next-line no-restricted-syntax
    for (const addr of followings) {
      // eslint-disable-next-line no-await-in-loop
      await this.syncLinks(myAddress!, addr, signal);
    }
  }

  public async syncLinks(myAddress: NeuronAddress, address: NeuronAddress, signal: AbortSignal) {
    let syncUpdates = [];
    try {
      this.statusApi.sendStatus(
        'in-progress',
        `starting sync ${address}...`,
        this.progressTracker.progress
      );
      const { timestampRead, unreadCount, meta } = await this.db!.getSyncStatus(myAddress, address);

      const { timestampUpdateChat = 0, timestampUpdateContent = 0 } = meta || {};

      const timestampFrom = timestampUpdateContent + 1; // ofsset + 1 to fix milliseconds precision bug

      const linksAsyncIterable = await fetchCyberlinksByNerounIterable(
        address,
        SENSE_FRIEND_PARTICLES,
        timestampFrom,
        CYBERLINKS_BATCH_LIMIT,
        signal
      );

      // eslint-disable-next-line no-restricted-syntax
      for await (const linksBatch of linksAsyncIterable) {
        this.statusApi.sendStatus(
          'in-progress',
          `sync ${address}...`,
          this.progressTracker.trackProgress(1)
        );

        const links = linksBatch.map(mapLinkFromIndexerToDto);

        const { timestampRead: newTimestampRead, unreadCount: newUnreadCount } = getLastReadInfo(
          links,
          myAddress,
          timestampRead,
          unreadCount
        );

        // const unreadItemsCount = unreadCount + links.length;

        if (links.length > 0) {
          const lastLink = entityToDto(links.at(-1)!);
          const newTimestampUpdateContent = lastLink!.timestamp;

          await throwIfAborted(this.db!.putCyberlinks, signal)(links);

          const particles = links.map((t) => t.to);
          await this.particlesResolver!.enqueueBatch(
            particles,
            SyncQueueJobType.particle,
            QueuePriority.HIGH
          );

          const newSyncItem = {
            ownerId: myAddress,
            entryType: EntryType.chat,
            id: address,
            timestampUpdate: Math.max(newTimestampUpdateContent, timestampUpdateChat),
            unreadCount: newUnreadCount,
            timestampRead: newTimestampRead,
            disabled: false,
            meta: {
              ...lastLink!,
              timestampUpdateContent: newTimestampUpdateContent,
              timestampUpdateChat,
            } as SenseItemLinkMeta,
          };
          // Update transaction
          await throwIfAborted(this.db!.putSyncStatus, signal)(newSyncItem);

          syncUpdates.push(newSyncItem);
        }
      }
    } catch (err) {
      this.cyblogCh.error(`>>> SyncMyFriends ${address} error`, {
        error: err,
      });
      if (!isAbortException(err)) {
        this.statusApi.sendStatus('error', err.toString());
      } else {
        syncUpdates = [];
        throw err;
      }
    } finally {
      // console.log('-----syncUpdates with redux', syncUpdates);
      this.channelApi.postSenseUpdate(syncUpdates);
    }
  }

  // eslint-disable-next-line class-methods-use-this
  // protected createRestartObserver(
  //   params$: Observable<SyncServiceParams>
  // ): Observable<boolean> {
  //   return super
  //     .createRestartObserver(params$)
  //     .pipe(switchMap((addressChanged) => this.isInitialized$));
  // }
}

export default SyncMyFriendsLoop;
