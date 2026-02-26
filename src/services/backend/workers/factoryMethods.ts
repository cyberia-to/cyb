import { expose, proxy, Remote, releaseProxy, transferHandlers, wrap } from 'comlink';
import { Observable, Observer, Subscribable, Subscription } from 'rxjs'; // v7.8.0
import { IPFSContentTransferHandler } from './serializers';

type WorkerType = SharedWorker | Worker;

const isSharedWorkersSupported = typeof SharedWorker !== 'undefined';

const isSharedWorkerUsed =
  isSharedWorkersSupported && !process.env.IS_DEV && !process.env.IS_TAURI;

// apply serializers for custom types
export function installTransferHandlers() {
  transferHandlers.set('IPFSContent', IPFSContentTransferHandler);
  transferHandlers.set('observable', {
    canHandle: (value: unknown): value is Observable<unknown> => {
      return value instanceof Observable;
    },
    deserialize: (value: MessagePort) => {
      return new Observable<unknown>((observer) => {
        const remote = transferHandlers.get('proxy')!.deserialize(value) as Remote<
          Subscribable<unknown>
        >;

        remote
          .subscribe(
            proxy({
              next: (next: unknown) => observer.next(next),
              error: (error: unknown) => observer.error(error),
              complete: () => observer.complete(),
            })
          )
          .then((subscription) =>
            observer.add(() => {
              subscription.unsubscribe();
              remote[releaseProxy]();
            })
          );
      });
    },
    serialize: (value: Observable<unknown>) => {
      return transferHandlers.get('proxy')!.serialize({
        subscribe: (observer: Remote<Observer<unknown>>) =>
          value.subscribe({
            next: (next: unknown) => observer.next(next).then(),
            error: (error: unknown) => observer.error(typeof error === 'string' ? error : String(error)).then(),
            complete: () => observer.complete().then(),
          }),
      });
    },
  });

  transferHandlers.set('subscription', {
    canHandle: (value: unknown): value is Subscription => {
      return value instanceof Subscription;
    },
    deserialize: (value: MessagePort) => {
      return new Subscription(() => {
        const remote = transferHandlers.get('proxy')!.deserialize(value) as Remote<Subscription>;

        remote.unsubscribe().then(() => {
          remote[releaseProxy]();
        });
      });
    },
    serialize: (value: Subscription) => {
      return transferHandlers.get('proxy')!.serialize({
        unsubscribe: () => value.unsubscribe(),
      });
    },
  });
}

function safeStringify(obj: any): string {
  try {
    return JSON.stringify(obj);
  } catch (_error) {
    return String(obj);
  }
}

// Override console.log to send logs to main thread
function overrideLogging(worker: Worker | MessagePort) {
  const consoleLogMap = {
    log: { original: console.log },
    error: { original: console.error },
    warn: { original: console.warn },
  };
  const replaceConsoleLog = (method: keyof typeof consoleLogMap) => {
    const { original } = consoleLogMap[method];

    consoleLogMap[method].original = console[method];

    console[method] = (...args) => {
      original.apply(console, args);
      const serializableArgs = args.map((arg) => safeStringify(arg));

      worker.postMessage({ type: 'console', method, args: serializableArgs });
    };
  };

  Object.keys(consoleLogMap).forEach((method) =>
    replaceConsoleLog(method as keyof typeof consoleLogMap)
  );
}

// Install handlers for logging from worker
function installLoggingHandler(worker: Worker | MessagePort, name: string) {
  // Add event listener
  worker.addEventListener('message', (event) => {
    if (event.data.type === 'console') {
      const { method, args } = event.data;

      console[method](name, ...args);
    }
  });
}

// Create Shared Worker with fallback to usual Worker(in case of DEV too)
export function createWorkerApi<T>(
  workerUrl: URL,
  workerName: string
): { worker: WorkerType; workerApiProxy: Remote<T> } {
  installTransferHandlers();
  // && !process.env.IS_DEV
  if (isSharedWorkerUsed) {
    const worker = new SharedWorker(workerUrl, { name: workerName });
    installLoggingHandler(worker.port, workerName);
    return { worker, workerApiProxy: wrap<T>(worker.port) };
  }

  const worker = new Worker(workerUrl);
  // installLoggingHandler(worker, workerName);
  return { worker, workerApiProxy: wrap<T>(worker) };
}


// Wrap already-created worker with comlink proxy (Worker must be created inline for rspack bundling)
export function createWorkerFromInstance<T>(
  worker: Worker,
  workerName: string
): { worker: WorkerType; workerApiProxy: Remote<T> } {
  installTransferHandlers();
  // installLoggingHandler(worker, workerName);
  return { worker, workerApiProxy: wrap<T>(worker) };
}

export function exposeWorkerApi<T>(worker: WorkerType, api: T) {
  installTransferHandlers();
  if (typeof worker.onconnect !== 'undefined') {
    worker.onconnect = (e) => {
      const port = e.ports[0];
      overrideLogging(port);

      expose(api, port);
    };
  } else {
    // Don't use overrideLogging for regular workers - it calls self.postMessage()
    // which interferes with comlink's message handling. Use BroadcastChannel instead.
    expose(api);
  }
}
