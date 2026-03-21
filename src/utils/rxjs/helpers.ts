/* eslint-disable import/prefer-default-export */
import { Observable } from 'rxjs';

/**
 * Convert promise to observable
 * @param promiseFactory
 * @returns
 */
export function promiseToObservable<T>(promiseFactory: () => Promise<T>) {
  return new Observable<T>((observer) => {
    promiseFactory()
      .then((response) => {
        observer.next(response);
        observer.complete();
      })
      .catch((error) => {
        console.debug('----promiseToObservable error', error); //, error
        observer.error(error);
      });
  });
}
