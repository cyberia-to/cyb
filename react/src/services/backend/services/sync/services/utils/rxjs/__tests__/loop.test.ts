import { defer, from, of, Subscription } from 'rxjs';
import { createLoopObservable } from '../loop';

describe('createLoopObservable', () => {
  let subscription: Subscription;
  afterEach(() => {
    subscription.unsubscribe();
  });

  it('should handle restart feature correctly', (done) => {
    const isInitialized$ = of(true);
    let taskCounter = 0;
    let initCounter = 0;
    const beforeCallback = () => initCounter++;

    const taskPromise = new Promise((resolve) => {
      taskCounter++;

      setTimeout(() => {
        resolve(true);
      }, 200);
    });

    const actionObservable$ = defer(() => from(taskPromise));

    const { loop$, restart } = createLoopObservable(isInitialized$, actionObservable$, {
      onStartInterval: beforeCallback,
      intervalMs: 100,
    });

    // restart
    setTimeout(() => restart(), 100);

    subscription = loop$.subscribe(() => {
      expect(taskCounter).toEqual(1);
      expect(initCounter).toEqual(2);
      done();
    });
  });

  it('should retry on exception', (done) => {
    const isInitialized$ = of(true);
    let beforeCounter = 0;
    let afterCounter = 0;

    const taskPromiseFactory = () =>
      new Promise((resolve) => {
        beforeCounter++;
        if (beforeCounter <= 1) {
          throw new Error('test error');
        }
        afterCounter++;
        resolve(true);
      });

    const actionObservable$ = defer(() => taskPromiseFactory());

    const { loop$ } = createLoopObservable(isInitialized$, actionObservable$, {
      intervalMs: 100,
    });

    subscription = loop$.subscribe(() => {
      if (afterCounter === 1) {
        expect(beforeCounter).toEqual(2);
        done();
      }
    });
  });
});
