import cozoDb from 'src/services/CozoDb/cozoDb';
import { exposeWorkerApi } from '../factoryMethods';
import BroadcastChannelSender from '../../channels/BroadcastChannelSender';
import migrate from 'src/services/CozoDb/migrations/migrations';
const createDbWorkerApi = ()=>{
    let isInitialized = false;
    const channel = new BroadcastChannelSender();
    const postServiceStatus = (status)=>channel.postServiceStatus('db', status);
    const init = async ()=>{
        postServiceStatus('starting');
        if (isInitialized) {
            console.log('cozo db - already initialized!');
            postServiceStatus('started');
            return Promise.resolve();
        }
        // callback to sync writes count worker -> main thread
        const onWriteCallback = (writesCount)=>{};
        // channel.post({ type: 'indexeddb_write', value: writesCount });
        console.time('🔋 cozo db initialized');
        await cozoDb.init(onWriteCallback);
        const reinitializeDbSchema = await migrate(cozoDb);
        if (reinitializeDbSchema) {
            await cozoDb.loadDbSchema();
        }
        console.timeEnd('🔋 cozo db initialized');
        isInitialized = true;
        setTimeout(()=>{
            postServiceStatus('started');
        }, 0);
        return Promise.resolve();
    };
    const runCommand = async (command, immutable)=>cozoDb.runCommand(command, immutable);
    const executePutCommand = async (tableName, array)=>cozoDb.put(tableName, array);
    const executeRmCommand = async (tableName, keyValues)=>cozoDb.rm(tableName, keyValues);
    const executeUpdateCommand = async (tableName, array)=>cozoDb.update(tableName, array);
    const executeGetCommand = async function(tableName, selectFields, conditions, conditionFields) {
        let options = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : {};
        return cozoDb.get(tableName, selectFields, conditions, conditionFields, options);
    };
    const importRelations = async (content)=>cozoDb.importRelations(content);
    const exportRelations = async (relations)=>cozoDb.exportRelations(relations);
    const executeBatchPutCommand = async function(tableName, array) {
        let batchSize = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : array.length, onProgress = arguments.length > 3 ? arguments[3] : void 0;
        const { getCommandFactory, runCommand } = cozoDb;
        const commandFactory = getCommandFactory();
        const putCommand = commandFactory.generateModifyCommand(tableName, 'put');
        for(let i = 0; i < array.length; i += batchSize){
            const batch = array.slice(i, i + batchSize);
            const atomCommand = commandFactory.generateAtomCommand(tableName, batch);
            // eslint-disable-next-line no-await-in-loop
            await runCommand([
                atomCommand,
                putCommand
            ].join('\r\n'));
            onProgress && onProgress(i + batch.length);
        }
        return {
            ok: true
        };
    };
    return {
        isInitialized: async ()=>isInitialized,
        init,
        runCommand,
        executeRmCommand,
        executePutCommand,
        executeBatchPutCommand,
        executeUpdateCommand,
        executeGetCommand,
        importRelations,
        exportRelations
    };
};
const cozoDbWorker = createDbWorkerApi();
// Expose the API to the main thread as shared/regular worker
exposeWorkerApi(self, cozoDbWorker);
