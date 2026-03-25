export async function waitUntil(cond: () => boolean, timeoutDuration = 60000) {
  if (cond()) {
    return true;
  }

  const waitPromise = new Promise((resolve) => {
    const interval = setInterval(() => {
      if (cond()) {
        clearInterval(interval);
        resolve(true);
      }
    }, 10);
  });

  const timeoutPromise = new Promise((_, reject) => {
    setTimeout(() => {
      reject(new Error('waitUntil timed out!'));
    }, timeoutDuration);
  });

  return Promise.race([waitPromise, timeoutPromise]);
}

// eslint-disable-next-line import/no-unused-modules
export function makeCancellable<T extends (...args: any[]) => Promise<any>>(
  func: T,
  signal: AbortSignal
): (...funcArgs: Parameters<T>) => Promise<ReturnType<T>> {
  return async (...args: Parameters<T>): Promise<ReturnType<T>> => {
    // Promise that listens for the abort signal
    const abortPromise = new Promise<ReturnType<T>>((_, reject) => {
      const abortHandler = () => {
        signal.removeEventListener('abort', abortHandler); // Clean up the event listener
        reject(new DOMException('The operation was aborted.', 'AbortError'));
      };
      signal.addEventListener('abort', abortHandler, { once: true });
    });

    // Wrapping the original function in a promise
    const taskPromise = new Promise<ReturnType<T>>(async (resolve, reject) => {
      try {
        const result = await func(...args);
        resolve(result);
      } catch (error) {
        reject(error);
      }
    });

    // Using Promise.race to handle cancellation
    return Promise.race([taskPromise, abortPromise]);
  };
}

export function throwIfAborted<T extends (...args: any[]) => Promise<any>>(
  func: T,
  signal: AbortSignal
): (...funcArgs: Parameters<T>) => Promise<ReturnType<T>> {
  return async (...args: Parameters<T>): Promise<ReturnType<T>> => {
    if (signal.aborted) {
      throw new DOMException('The operation was aborted.', 'AbortError');
    }
    return func(...args);
  };
}

/**
 * Promise will be rejected after timeout.
 *
 * @param promise
 * @param timeout ms
 * @param abortController trigger abort
 * @returns
 */
// eslint-disable-next-line import/no-unused-modules
export async function withTimeout<T>(
  promise: Promise<T>,
  timeout: number,
  abortController?: AbortController
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => {
      const timer = setTimeout(() => {
        abortController?.abort('timeout');
        clearTimeout(timer);
        reject(new DOMException('timeout', 'AbortError'));
      }, timeout);
    }),
  ]);
}
