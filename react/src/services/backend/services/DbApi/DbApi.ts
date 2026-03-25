import { SenseListItem } from 'src/services/backend/types/sense';
import { CozoDbWorker } from 'src/services/backend/workers/db/worker';
import {
  CommunityDto,
  LinkDto,
  ParticleDto,
  SyncQueueDto,
  SyncQueueKeyDto,
  SyncStatusDto,
  TransactionDto,
} from 'src/services/CozoDb/types/dto';
import {
  EmbeddinsDbEntity,
  EntryType,
  PinDbEntity,
  SyncQueueStatus,
} from 'src/services/CozoDb/types/entities';
import { dbResultToDtoList, toListOfObjects } from 'src/services/CozoDb/utils';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { dtoListToEntity, dtoToEntity, removeUndefinedFields } from 'src/utils/dto';
import { MSG_MULTI_SEND_TRANSACTION_TYPE, MSG_SEND_TRANSACTION_TYPE } from '../indexer/types';
import { SyncQueueItem } from '../sync/services/ParticlesResolverQueue/types';
import { extractSenseChats } from '../sync/services/utils/sense';

const TIMESTAMP_INTITAL = 0;
const DEFAULT_SYNC_STATUS = {
  timestampUpdate: TIMESTAMP_INTITAL,
  timestampRead: TIMESTAMP_INTITAL,
  unreadCount: 0,
  meta: {},
} as SyncStatusDto;

class DbApiWrapper {
  private db: CozoDbWorker | undefined;

  public init(dbApi: CozoDbWorker) {
    this.db = dbApi;
    return this;
  }

  public async getSyncStatus(
    ownerId: NeuronAddress,
    id: NeuronAddress | ParticleCid,
    defaultSyncStatus = DEFAULT_SYNC_STATUS
  ) {
    const result = await this.db!.executeGetCommand(
      'sync_status',
      undefined,
      [`id = '${id}'`, `owner_id = '${ownerId}'`],
      ['id', 'owner_id']
    );

    return result.rows.length ? dbResultToDtoList<SyncStatusDto>(result)[0] : defaultSyncStatus;
  }

  public async putSyncStatus(item: SyncStatusDto[] | SyncStatusDto) {
    const entitites = dtoListToEntity(Array.isArray(item) ? item : [item]);
    const result = await this.db!.executePutCommand('sync_status', entitites);

    return result;
  }

  public async updateSyncStatus(
    entity: Partial<SyncStatusDto> & {
      id: NeuronAddress | ParticleCid;
      ownerId: NeuronAddress;
    }
  ) {
    return this.db!.executeUpdateCommand('sync_status', [
      dtoToEntity(removeUndefinedFields(entity)),
    ]);
  }

  public async findSyncStatus({
    ownerId,
    entryType,
    id,
  }: {
    ownerId: NeuronAddress;
    entryType?: EntryType[] | EntryType;
    id?: NeuronAddress | ParticleCid;
  }) {
    const conditions = [`owner_id = '${ownerId}'`];

    entryType &&
      conditions.push(
        `entry_type in [${Array.isArray(entryType) ? entryType.join(', ') : entryType}]`
      );

    id && conditions.push(`id = '${id}'`);

    const result = await this.db!.executeGetCommand(
      'sync_status',
      ['id', 'unread_count', 'timestamp_update', 'timestamp_read', 'meta'],
      conditions,
      ['entry_type', 'owner_id'],
      { orderBy: ['-timestamp_update'] }
    );
    return dbResultToDtoList<SyncStatusDto>(result);
  }

  public async putTransactions(transactions: TransactionDto[]) {
    return this.db!.executePutCommand('transaction', dtoListToEntity(transactions));
  }

  public async putPins(pins: PinDbEntity[] | PinDbEntity) {
    const entitites = Array.isArray(pins) ? pins : [pins];
    return this.db!.executePutCommand('pin', entitites);
  }

  public async getPins(withType = false) {
    const fields = withType ? ['cid', 'type'] : ['cid'];
    const result = await this.db!.executeGetCommand('pin', fields);

    return result;
  }

  public async getCommunity(ownerId: NeuronAddress) {
    const result = await this.db!.executeGetCommand('community', undefined, [
      `owner_id = '${ownerId}'`,
    ]);

    return dbResultToDtoList<CommunityDto>(result);
  }

  public async putCommunity(community: CommunityDto[] | CommunityDto) {
    const entitites = dtoListToEntity(Array.isArray(community) ? community : [community]);
    await this.db!.executePutCommand('community', entitites);
  }

  public async deletePins(pins: ParticleCid[]) {
    return this.db!.executeRmCommand(
      'pin',
      pins.map((cid) => ({ cid }) as Partial<PinDbEntity>)
    );
  }

  public async putParticles(particles: ParticleDto[] | ParticleDto) {
    const entitites = dtoListToEntity(Array.isArray(particles) ? particles : [particles]);
    return this.db!.executePutCommand('particle', entitites);
  }

  public async getParticlesRaw(fields: string[]) {
    const result = await this.db!.executeGetCommand('particle', fields);

    return result;
  }

  public async getSenseList(myAddress: NeuronAddress = '') {
    const command = `
    ss_particles[id, meta] := *sync_status{entry_type,id, meta, owner_id}, entry_type=${EntryType.particle}, owner_id = '${myAddress}'

    ss_chat_all[id, meta, hash, is_link, index] := *sync_status{entry_type, id, meta, owner_id}, entry_type=${EntryType.chat}, index=maybe_get(meta, 'index'), hash=maybe_get(meta, 'transaction_hash'), is_link=!is_null(maybe_get(meta, 'to')), owner_id = '${myAddress}'
    ss_chat_links[id, meta] := ss_chat_all[id, meta, hash, is_link, index], is_link
    ss_chat_trans[id, m] := ss_chat_all[id, meta, hash, is_link, index], !is_link, *transaction{hash, index, value, type, timestamp, success, memo},  m=concat(meta, json_object('value', value, 'type', type, 'timestamp', timestamp, 'memo', memo, 'success', success, 'index', index))

    ?[owner_id, entry_type, id, unread_count, timestamp_update, timestamp_read, meta, meta_timestamp] := *sync_status{entry_type, id, unread_count, timestamp_update, timestamp_read, owner_id}, ss_particles[id, meta] or ss_chat_links[id, meta] or ss_chat_trans[id, meta], id!='${myAddress}', owner_id = '${myAddress}', meta_timestamp = maybe_get(meta, 'timestamp')
    :order -meta_timestamp
    `;

    const result = await this.db!.runCommand(command, true);
    const senseList = dbResultToDtoList<SenseListItem>(result);

    return senseList;
  }

  public async getTransactions(
    neuron: NeuronAddress,
    { timestampFrom = 0, order = 'desc' }: { order?: 'desc' | 'asc'; timestampFrom?: number } = {}
  ) {
    const conditions = [`neuron = '${neuron}'`, `timestamp >= ${timestampFrom}`];
    const result = await this.db!.executeGetCommand(
      'transaction',
      ['hash', 'type', 'success', 'value', 'timestamp', 'memo', 'index'],
      conditions,
      ['neuron'],
      { orderBy: [order === 'desc' ? '-timestamp' : 'timestamp'] }
    );

    return dbResultToDtoList<TransactionDto>(result);
  }

  public async getMyChats(myAddress: NeuronAddress, userAddress: NeuronAddress) {
    const result = await this.db!.executeGetCommand(
      'transaction',
      ['hash', 'type', 'success', 'value', 'timestamp', 'memo'],
      [
        `neuron = '${myAddress}', type='${MSG_SEND_TRANSACTION_TYPE}' or type='${MSG_MULTI_SEND_TRANSACTION_TYPE}'`,
      ],
      ['neuron'],
      { orderBy: ['timestamp'] }
    );

    const sendTransactions = dbResultToDtoList<TransactionDto>(result);

    const chats = extractSenseChats(myAddress, sendTransactions);
    const userChats = [...chats.values()].find((c) => c.userAddress === userAddress);

    return userChats ? userChats.transactions : [];
  }

  public async putCyberlinks(links: LinkDto[] | LinkDto) {
    const entitites = dtoListToEntity(Array.isArray(links) ? links : [links]);
    return this.db!.executePutCommand('link', entitites);
  }

  public async putSyncQueue(item: SyncQueueItem[] | SyncQueueItem) {
    const entitites = dtoListToEntity(Array.isArray(item) ? item : [item]);
    return this.db!.executePutCommand('sync_queue', entitites);
  }

  public async existSyncQueueItem({ id, jobType }: SyncQueueKeyDto) {
    const result = await this.db!.executeGetCommand(
      'sync_queue',
      ['id'],
      [`id = '${id}'`, `job_type = ${jobType}`],
      ['id', 'job_type']
    );

    return result.rows.length > 0;
  }

  public async existEmbedding(cid: ParticleCid) {
    const result = await this.db!.executeGetCommand('embeddings', ['cid'], [`cid = '${cid}'`]);

    return result.rows.length > 0;
  }

  public async updateSyncQueue(item: Partial<SyncQueueDto>[] | Partial<SyncQueueDto>) {
    const entitites = dtoListToEntity(Array.isArray(item) ? item : [item]);
    try {
      return this.db!.executeUpdateCommand('sync_queue', entitites);
    } catch (e) {
      cyblog.error('----> updateSyncQueue, items will be removed', {
        error: e,
        data: item,
        module: 'dbApiWrapper',
      });
      // eslint-disable-next-line no-restricted-syntax
      for (const entity of entitites) {
        if (entity) {
          const { id, job_type: jobType = 0 } = entity;
          // eslint-disable-next-line no-await-in-loop
          await this.removeSyncQueue({ id: id!, jobType });
        }
      }

      return { ok: true };
    }
  }

  public async removeSyncQueue(id: SyncQueueKeyDto | SyncQueueKeyDto[]) {
    const ids = dtoListToEntity(Array.isArray(id) ? id : [id]);

    return this.db!.executeRmCommand('sync_queue', ids);
  }

  public async getSyncQueue({
    statuses,
    limit,
  }: {
    statuses: SyncQueueStatus[];
    limit?: number;
  }): Promise<SyncQueueDto[]> {
    const result = await this.db!.executeGetCommand(
      'sync_queue',
      ['id', 'status', 'priority', 'job_type'],
      [`status in [${statuses.join(',')}]`],
      [],
      { orderBy: ['-priority'], limit }
    );

    return result.rows.map((row) => ({
      id: row[0] as string,
      status: row[1] as SyncQueueStatus,
      priority: row[2] as number,
      jobType: row[3] as number,
    }));
  }

  public async getLinks({
    cid,
    neuron,
    timestampFrom,
  }: {
    cid?: ParticleCid | ParticleCid[];
    neuron?: NeuronAddress;
    timestampFrom?: number;
  }) {
    const conditions = [];

    if (cid) {
      if (!Array.isArray(cid)) {
        conditions.push(`from='${cid}' or to='${cid}'`);
      } else {
        const listStr = cid.map((i) => `'${i}'`).join(', ');
        conditions.push(`is_in(from,[${listStr}]) or is_in(to,[${listStr}])`);
      }
    }

    neuron && conditions.push(`neuron='${neuron}'`);
    timestampFrom && conditions.push(`timestamp > ${timestampFrom}`);

    const result = await this.db!.executeGetCommand(
      'link',
      ['from', 'to', 'timestamp', 'neuron', 'transaction_hash'],
      conditions,
      [],
      { orderBy: ['timestamp'] }
    );

    return dbResultToDtoList<LinkDto>(result);
  }

  public async searchByEmbedding(vec: number[], count?: number) {
    const queryText = `
    e[dist, cid] := ~embeddings:semantic{cid | query: vec([${vec}]), bind_distance: dist, k: 20, ef: 50}
    ?[dist, cid, text] := e[dist, cid], *particle{cid, text}
    ${count ? `\r\n:limit ${count}` : ''}`;

    const result = await this.db!.runCommand(queryText);
    return toListOfObjects(result);
  }

  public async putEmbedding(cid: string, vec: number[]) {
    return this.db!.executePutCommand('embeddings', [
      {
        cid,
        vec,
      } as EmbeddinsDbEntity,
    ]);
  }
}

export default DbApiWrapper;
