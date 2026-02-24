/* eslint-disable no-restricted-syntax */
import { combineLatest, Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { createCyblogChannel } from 'src/utils/logging/cyblog';
import BackendQueueChannelListener from '../../channels/BackendQueueChannel/BackendQueueChannel';
import BroadcastChannelSender from '../../channels/BroadcastChannelSender';
import { SyncEntryName } from '../../types/services';
import BaseSyncLoop from './services/BaseSyncLoop/BaseSyncLoop';
import createCommunitySync$ from './services/CommunitySync/CommunitySync';
import { MY_FRIENDS_SYNC_INTERVAL, MY_PARTICLES_SYNC_INTERVAL } from './services/consts';
import ParticlesResolverQueue from './services/ParticlesResolverQueue/ParticlesResolverQueue';
import SyncMyFriendsLoop from './services/SyncMyFriendsLoop/SyncMyFriendsLoop';
import SyncParticlesLoop from './services/SyncParticlesLoop/SyncParticlesLoop';
// import SyncIpfsLoop from './services/SyncIpfsLoop/SyncIpfsLoop';
import SyncTransactionsLoop from './services/SyncTransactionsLoop/SyncTransactionsLoop';
import { ServiceDeps } from './services/types';

const cyblogCh = createCyblogChannel({ thread: 'bckd' });

// eslint-disable-next-line import/prefer-default-export
export class SyncService {
  private isInitialized$: Observable<boolean>;

  private channelApi = new BroadcastChannelSender();

  private loops: Partial<Record<SyncEntryName, BaseSyncLoop>> = {};

  constructor(deps: ServiceDeps) {
    const { dbInstance$, ipfsInstance$ } = deps;

    const particlesResolver = new ParticlesResolverQueue(deps).start();

    const _queueListener = new BackendQueueChannelListener(particlesResolver, dbInstance$);

    this.isInitialized$ = combineLatest([dbInstance$, ipfsInstance$]).pipe(
      map(([dbInstance, ipfsInstance]) => !!dbInstance && !!ipfsInstance)
    );
    // subscribe when started
    this.isInitialized$.subscribe({
      next: (result) => {
        return result && this.channelApi.postServiceStatus('sync', 'started');
      },
      error: (err) => this.channelApi.postServiceStatus('sync', 'error', err),
    });

    const communitySync$ = createCommunitySync$(deps);
    communitySync$.subscribe((community) => {
      cyblogCh.info('--> community fetched', {
        unit: 'community',
        data: community,
      });
    });

    const followings$ = communitySync$.pipe(
      map((c) => c.filter((i) => i.following)),
      map((c) => c.map((i) => i.neuron))
    );

    // new SyncIpfsLoop(deps, particlesResolver).start();

    new SyncTransactionsLoop('transactions', deps, particlesResolver).start();

    new SyncParticlesLoop('particles', MY_PARTICLES_SYNC_INTERVAL, deps, particlesResolver).start();

    new SyncMyFriendsLoop(
      'my-friends',
      MY_FRIENDS_SYNC_INTERVAL,
      { ...deps, followings$ },
      particlesResolver
      // { warmupMs: 1000 }
    ).start();
  }

  public restart(name: SyncEntryName) {
    this.loops[name]?.restart();
  }
}
