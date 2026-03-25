import { distinctUntilChanged, filter, map, Observable, switchMap } from 'rxjs';

import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { broadcastStatus } from 'src/services/backend/channels/broadcastStatus';
import { SyncEntryName } from 'src/services/backend/types/services';
import { CyblogChannel, createCyblogChannel } from 'src/utils/logging/cyblog';

import DbApiWrapper from '../../../DbApi/DbApi';
import { SyncServiceParams } from '../../types';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ProgressTracker } from '../ProgressTracker/ProgressTracker';
import { ServiceDeps } from '../types';

abstract class BaseSync {
  protected name: string;

  protected abortController: AbortController;

  protected db: DbApiWrapper | undefined;

  protected progressTracker = new ProgressTracker();

  protected channelApi = new BroadcastChannelSender();

  protected particlesResolver: ParticlesResolverQueue | undefined;

  protected statusApi: ReturnType<typeof broadcastStatus>;

  protected params: SyncServiceParams = {
    myAddress: null,
  };

  protected readonly isInitialized$: Observable<boolean>;

  protected cyblogCh: CyblogChannel;

  constructor(name: SyncEntryName, deps: ServiceDeps, particlesResolver: ParticlesResolverQueue) {
    this.name = name;

    this.abortController = new AbortController();

    this.statusApi = broadcastStatus(name, this.channelApi);
    this.particlesResolver = particlesResolver;
    this.cyblogCh = createCyblogChannel({ thread: 'bckd', module: name });
    if (!deps.params$) {
      throw new Error('params$ is not defined');
    }

    deps.dbInstance$.subscribe((db) => {
      this.db = db;
    });

    this.particlesResolver = particlesResolver;

    this.isInitialized$ = this.createIsInitializedObserver(deps);

    this.isInitialized$.subscribe((isInitialized) => {
      this.cyblogCh.info(`>>> ${this.name} - ${isInitialized ? 'initialized' : 'inactive'}`);
      this.statusApi.sendStatus(isInitialized ? 'initialized' : 'inactive');
    });

    this.isInitialized$.pipe(switchMap(() => deps.params$!)).subscribe((params) => {
      this.params = params;
      this.cyblogCh.info(`>>> ${this.name} - params updated`, {
        data: params,
      });
    });

    // Restart observer
    this.isInitialized$
      .pipe(
        filter((isInitialized) => !!isInitialized),
        switchMap(() => this.createRestartObserver(deps.params$!))
      )
      .subscribe(() => {
        this.restart();
      });
  }

  protected initAbortController() {
    this.abortController = new AbortController();
  }

  protected abstract createIsInitializedObserver(deps: ServiceDeps): Observable<boolean>;

  // eslint-disable-next-line class-methods-use-this
  protected createRestartObserver(params$: Observable<SyncServiceParams>) {
    return params$.pipe(
      map((params) => params.myAddress),
      distinctUntilChanged((addrBefore, addrAfter) => addrBefore === addrAfter),
      map((v) => !!v),
      filter((v) => !!v)
    );
  }

  public abstract restart(): void;

  public abstract start(): void;
}

export default BaseSync;
