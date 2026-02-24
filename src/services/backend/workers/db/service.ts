import { wrap } from 'comlink';
import { installTransferHandlers } from '../factoryMethods';
import type { CozoDbWorker } from './worker';
// worker-rspack-loader transforms this import into a Worker constructor
// @ts-expect-error default export is added by worker-rspack-loader
import DbWorkerCtor from './worker';

installTransferHandlers();
const worker: Worker = new DbWorkerCtor();
export const cozoDbWorkerInstance = wrap<CozoDbWorker>(worker);
