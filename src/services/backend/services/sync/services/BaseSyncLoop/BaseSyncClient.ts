import { from, Observable, Subject, startWith, switchMap, tap } from 'rxjs';

import { SyncEntryName } from 'src/services/backend/types/services';
import { SyncServiceParams } from '../../types';
import ParticlesResolverQueue from '../ParticlesResolverQueue/ParticlesResolverQueue';
import { ServiceDeps } from '../types';
import { switchWhenInitialized } from '../utils/rxjs/withInitializer';
import BaseSync from './BaseSync';

abstract class BaseSyncClient extends BaseSync {
  protected readonly source$: Observable<any>;

  protected readonly reloadTrigger$ = new Subject<void>();

  constructor(name: SyncEntryName, deps: ServiceDeps, particlesResolver: ParticlesResolverQueue) {
    super(name, deps, particlesResolver);

    const source$ = switchWhenInitialized(
      this.isInitialized$!,
      this.reloadTrigger$.pipe(
        startWith(null),
        tap(() => {
          // initialize abort conteoller for restart strategy
          this.initAbortController();
        }),
        switchMap(() =>
          this.createInitObservable().pipe(
            switchMap((timestampFrom: number) =>
              this.createClientObservable(timestampFrom).pipe(
                tap(() => this.statusApi.sendStatus('listen')),
                switchMap((data) => from(this.onUpdate(data, this.params)))
              )
            )
          )
        )
      ),
      (isInitialized) => {
        console.log(`>>> ${name} isInitialized`, isInitialized);
        this.statusApi.sendStatus(isInitialized ? 'initialized' : 'inactive');
      }
    );

    source$.subscribe({
      next: () => {
        this.statusApi.sendStatus('listen');
      },
      error: (err) => {
        this.statusApi.sendStatus('error', err);
      },
    });
    this.source$ = source$;
  }

  protected abstract createClientObservable(timestampFrom: number): Observable<any>;

  protected abstract createInitObservable(): Observable<number>;

  public restart() {
    this.abortController?.abort();
    this.reloadTrigger$.next();
    console.log(`>>> ${this.name} client restart`);
  }

  protected abstract onUpdate(data: any, params: SyncServiceParams): Promise<void>;

  public start() {
    this.source$.subscribe(() => {
      // dummy subscriber to keep pipeline running - don't remove
    });
    return this;
  }
}

export default BaseSyncClient;
