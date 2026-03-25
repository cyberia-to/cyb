import cozoDb from 'src/services/CozoDb/cozoDb';
import migrate from 'src/services/CozoDb/migrations/migrations';
import { DbEntity } from 'src/services/CozoDb/types/entities';
import { GetCommandOptions } from 'src/services/CozoDb/types/types';
import BroadcastChannelSender from '../../channels/BroadcastChannelSender';
import { ServiceStatus } from '../../types/services';
import { exposeWorkerApi } from '../factoryMethods';

const createDbWorkerApi = () => {
  let isInitialized = false;
  const channel = new BroadcastChannelSender();

  const postServiceStatus = (status: ServiceStatus) => channel.postServiceStatus('db', status);

  const init = async () => {
    postServiceStatus('starting');

    if (isInitialized) {
      console.log('cozo db - already initialized!');
      postServiceStatus('started');
      return Promise.resolve();
    }

    // callback to sync writes count worker -> main thread
    const onWriteCallback = (_writesCount: number) => {};
    // channel.post({ type: 'indexeddb_write', value: writesCount });
    console.time('🔋 cozo db initialized');

    await cozoDb.init(onWriteCallback);
    const reinitializeDbSchema = await migrate(cozoDb);
    if (reinitializeDbSchema) {
      await cozoDb.loadDbSchema();
    }
    console.timeEnd('🔋 cozo db initialized');

    isInitialized = true;

    setTimeout(() => {
      postServiceStatus('started');
    }, 0);
    return Promise.resolve();
  };

  const runCommand = async (command: string, immutable?: boolean) =>
    cozoDb.runCommand(command, immutable);

  const executePutCommand = async (tableName: string, array: Partial<DbEntity>[]) =>
    cozoDb.put(tableName, array);

  const executeRmCommand = async (tableName: string, keyValues: Partial<DbEntity>[]) =>
    cozoDb.rm(tableName, keyValues);

  const executeUpdateCommand = async (tableName: string, array: Partial<DbEntity>[]) =>
    cozoDb.update(tableName, array);

  const executeGetCommand = async (
    tableName: string,
    selectFields?: string[],
    conditions?: string[],
    conditionFields?: string[],
    options: GetCommandOptions = {}
  ) => cozoDb.get(tableName, selectFields, conditions, conditionFields, options);

  const importRelations = async (content: string) => cozoDb.importRelations(content);

  const exportRelations = async (relations: string[]) => cozoDb.exportRelations(relations);

  const executeBatchPutCommand = async (
    tableName: string,
    array: Partial<DbEntity>[],
    batchSize: number = array.length,
    onProgress?: (count: number) => void
  ) => {
    const { getCommandFactory, runCommand } = cozoDb;

    const commandFactory = getCommandFactory();
    const putCommand = commandFactory!.generateModifyCommand(tableName, 'put');
    for (let i = 0; i < array.length; i += batchSize) {
      const batch = array.slice(i, i + batchSize);

      const atomCommand = commandFactory!.generateAtomCommand(tableName, batch);

      // eslint-disable-next-line no-await-in-loop
      await runCommand([atomCommand, putCommand].join('\r\n'));

      onProgress?.(i + batch.length);
    }
    return { ok: true };
  };

  return {
    isInitialized: async () => isInitialized,
    init,
    runCommand,
    executeRmCommand,
    executePutCommand,
    executeBatchPutCommand,
    executeUpdateCommand,
    executeGetCommand,
    importRelations,
    exportRelations,
  };
};
const cozoDbWorker = createDbWorkerApi();

export type CozoDbWorker = typeof cozoDbWorker;

// Expose the API to the main thread as shared/regular worker
exposeWorkerApi(self, cozoDbWorker);
