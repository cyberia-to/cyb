import { QueuePriority } from 'src/services/QueueManager/types';
import { ParticleCid } from 'src/types/base';
import { dtoListToEntity } from 'src/utils/dto';
import { getRelevance } from 'src/utils/search/utils';
import { type CybCozoDb, DB_VERSION } from '../cozoDb';
import { SyncQueueDto } from '../types/dto';
import { DbEntity, SyncQueueJobType } from '../types/entities';

export const fetchInitialEmbeddings = async (
  saveSyncQueue: (syncItems: Partial<DbEntity>[]) => Promise<void>
) => {
  console.log(' [initial]fetch initial particles...');
  const relevanceParticles = await getRelevance(0, 400);

  const items = relevanceParticles.result.map(({ particle }: { particle: ParticleCid }) => ({
    id: particle,
    data: '',
    jobType: SyncQueueJobType.particle,
    priority: QueuePriority.LOW,
  })) as SyncQueueDto[];

  await saveSyncQueue(dtoListToEntity(items));
};

const migrate = async (db: CybCozoDb) => {
  const version = await db.getDbVersion();
  console.log(`* db version - ${version}`);
  if (version < 1.2) {
    //
    console.log('⚡️ Migrating to 1.1');
    console.log('    add job_type field to sync_queue');
    const res1 = await db.runCommand(`
        ?[id,status,priority, job_type, data] := *sync_queue{id,status,priority}, job_type=0, data='';
        :replace sync_queue {
            id: String,
            job_type: Int default 0 =>
            data: String default '',
            status: Int default 0,
            priority: Float default 0,
        }
    `);
    console.log(`       ok: ${res1.ok}`);
    try {
      console.log('    create embeddings relation');
      const res2 = await db.runCommand(`
        :create embeddings {
            cid: String =>
            vec: <F32; 384>
        }
      `);

      console.log(`       ok: ${res2.ok}`);
    } catch (_e) {
      console.log('* embeddings already exist');
    }

    try {
      console.log('    create embeddings:semantic index');
      const res3 = await db.runCommand(`
        ::hnsw create embeddings:semantic{
            fields: [vec],
            dim: 384,
            ef: 100,
            m: 16
        }
    `);
      console.log(`       ok: ${res3.ok}`);
    } catch (_e) {
      console.log('* embeddings semantics already exist');
    }
    console.log('    fill queue to calculate embeddings for all particles');
    const res4 = await db.runCommand(`
        ?[id, data, job_type, status, priority] := *particle{cid, text, mime}, mime="text/plain", job_type=1, status=0, priority=0.5, id = cid, data=text

        :put sync_queue {
            id,
            job_type =>
            data,
            status,
            priority,
        }
    `);
    console.log(`       ok: ${res4.ok}`);
    await db.setDbVersion(DB_VERSION);

    return true;
  }

  return false;
};

export default migrate;
