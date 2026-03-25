/* eslint-disable import/prefer-default-export */
import { distinctUntilChanged, filter, Observable, share, switchMap, tap } from 'rxjs';

export const switchWhenInitialized = (
  isInitialized$: Observable<boolean>,
  actionObservable$: Observable<any>,
  onChange?: (isInitialized: boolean) => void
) =>
  isInitialized$.pipe(
    distinctUntilChanged(),
    tap((isInitialized) => onChange?.(isInitialized)),
    filter((initialized) => initialized),
    switchMap(() => actionObservable$),
    share()
  );
