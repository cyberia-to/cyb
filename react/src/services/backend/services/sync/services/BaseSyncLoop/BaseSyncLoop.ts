import { clone } from 'ramda';
import { defer, from, Observable } from 'rxjs';
import { SyncEntryName } from 'src/services/backend/types/services';
import { isAbortException } from 'src/utils/exceptions/helpers';
import { SyncServiceParams } from '../../types';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import { createLoopObservable } from '../utils/rxjs/loop';
import BaseSync from './BaseSync';

abstract class BaseSyncLoop extends BaseSync {
  private restartLoop: (() => void) | undefined;

  public readonly loop$: Observable<boolean>;

  constructor(
    name: SyncEntryName,
    intervalMs: number,
    deps: ServiceDeps,
    particlesResolver: ParticlesResolverQueue,
    {
      warmupMs,
    }: {
      warmupMs: number;
    } = { warmupMs: 0 }
  ) {
    super(name, deps, particlesResolver);

    const { loop$, restartLoop } = createLoopObservable(
      this.isInitialized$,
      // defer(() => from(this.sync())),
      defer(() => from(this.doSync())),
      {
        intervalMs,
        warmupMs,
        // onStartInterval: () => this.initAbortController(),
        onError: (error) => {
          this.cyblogCh.info(`>>> ${name} error`, error.toString());
          this.statusApi.sendStatus('error', error.toString());
        },
        onChange: (isInitialized) => {
          this.cyblogCh.info(`>>> ${name} initialized: ${isInitialized}`);
          this.statusApi.sendStatus(isInitialized ? 'initialized' : 'inactive');
        },
      }
    );

    this.loop$ = loop$;
    this.restartLoop = restartLoop;
  }

  public restart() {
    this.abortController?.abort();
    this.restartLoop?.();
    this.cyblogCh.info(`>>> ${this.name} loop restart`);
  }

  public start() {
    this.loop$.subscribe(() => this.statusApi.sendStatus('active'));
    return this;
  }

  private async doSync() {
    const params = clone(this.params);
    this.initAbortController();
    try {
      await this.sync(params);
    } catch (e) {
      const isAborted = isAbortException(e);
      this.cyblogCh.info(`>>> ${this.name} ${params.myAddress} sync error [abrt:${isAborted}]:`, {
        error: e,
      });

      if (!isAborted) {
        throw e;
      }
    }
  }

  protected abstract sync(params: SyncServiceParams): Promise<void>;
}

export default BaseSyncLoop;
