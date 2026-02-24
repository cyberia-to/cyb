import { wrap } from 'comlink';
import { installTransferHandlers } from '../factoryMethods';
import type { BackgroundWorker } from './worker';
// worker-rspack-loader transforms this import into a Worker constructor
// @ts-expect-error default export is added by worker-rspack-loader
import BackgroundWorkerCtor from './worker';

installTransferHandlers();
const worker: Worker = new BackgroundWorkerCtor();

// Catch worker errors
worker.addEventListener('error', (event) => {
  console.error('[bg-worker] Worker error:', event.message, event);
});

export const backgroundWorkerInstance = wrap<BackgroundWorker>(worker);
