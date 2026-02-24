/* eslint-disable import/prefer-default-export */
import {
  delay,
  exhaustMap,
  interval,
  Observable,
  retry,
  Subject,
  startWith,
  switchMap,
  tap,
} from 'rxjs';
import { switchWhenInitialized } from './withInitializer';

type LoopObservableOptions = {
  warmupMs?: number;
  retryDelayMs?: number;
  onStartInterval?: () => void;
  onError?: (error: any) => void;
  onChange?: (isInitialized: boolean) => void;
  intervalMs?: number;
};

export const createLoopObservable = (
  isInitialized$: Observable<boolean>,
  actionObservable$: Observable<any>,
  options: LoopObservableOptions = {}
) => {
  const {
    intervalMs,
    warmupMs = 0,
    onStartInterval,
    onError,
    retryDelayMs = 0,
    onChange,
  } = options;

  const restartTrigger$ = new Subject<void>();

  const intervalOrRestart$ = restartTrigger$.pipe(
    startWith(null),
    switchMap(() => interval(intervalMs).pipe(startWith(0), delay(warmupMs)))
  );

  const source$ = switchWhenInitialized(
    isInitialized$,
    intervalOrRestart$.pipe(
      tap(() => onStartInterval?.()),
      exhaustMap(() =>
        actionObservable$.pipe(
          retry({
            delay: (error) => {
              console.log('retry', error);
              onError?.(error);
              return interval(retryDelayMs);
            },
          })
        )
      )
    ),
    (isInitialized) => onChange?.(isInitialized)
  );

  return {
    loop$: source$,
    restartLoop: () => {
      // console.log('>>> createLoopObservable restart');
      // Trigger a restart by emitting a new value
      restartTrigger$.next();
    },
  };
};
