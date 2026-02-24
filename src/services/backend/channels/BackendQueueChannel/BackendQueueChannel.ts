import { BehaviorSubject, first, Observable } from 'rxjs';
import { mapParticleToEntity } from 'src/services/CozoDb/mapping';
import { LinkDto } from 'src/services/CozoDb/types/dto';
import { IPFSContent } from 'src/services/ipfs/types';
import { Option } from 'src/types';
import DbApi from '../../services/DbApi/DbApi';
import ParticlesResolverQueue from '../../services/sync/services/ParticlesResolverQueue/ParticlesResolverQueue';
import { SyncQueueItem } from '../../services/sync/services/ParticlesResolverQueue/types';
import { CYB_QUEUE_CHANNEL } from '../consts';
import { enqueueParticleEmbeddingMaybe } from './backendQueueSenders';
import { QueueChannelMessage } from './types';

class BackendQueueChannelListener {
  private channel = new BroadcastChannel(CYB_QUEUE_CHANNEL);

  private particlesResolver: ParticlesResolverQueue;

  private dbInstance$: BehaviorSubject<Option<DbApi>>;

  constructor(
    particlesResolver: ParticlesResolverQueue,
    dbInstance$: Observable<DbApi | undefined>
  ) {
    this.particlesResolver = particlesResolver;
    this.dbInstance$ = new BehaviorSubject<Option<DbApi>>(undefined);
    dbInstance$.subscribe((v) => {
      this.dbInstance$.next(v);
    });

    this.channel.onmessage = (event) => this.onMessage(event);

    this.channel.onmessageerror = (event) => console.error(`${CYB_QUEUE_CHANNEL} error`, event);
  }

  private async getDeffredDbApi(): Promise<DbApi> {
    return new Promise((resolve) => {
      const dbApi = this.dbInstance$.getValue();
      if (dbApi) {
        resolve(dbApi);
      }

      this.dbInstance$
        .pipe(
          first((value) => value !== undefined) // Automatically unsubscribes after the first valid value
        )
        .subscribe((value) => {
          resolve(value as DbApi);
        });
    });
  }

  private async saveLinks(links: LinkDto[]) {
    const dbApi = await this.getDeffredDbApi();
    const _res = await dbApi.putCyberlinks(links);
    // console.log('---saveLinks done', links, res);
  }

  private async saveParticles(content: IPFSContent) {
    try {
      const dbApi = await this.getDeffredDbApi();
      const entity = mapParticleToEntity(content);
      const result = await dbApi.putParticles(entity);
      if (result.ok) {
        await enqueueParticleEmbeddingMaybe(content);
      }
    } catch (e) {
      console.log('---saveParticle e', content, content.textPreview, e.toString());
      throw e;
    }
  }

  private async enquueSync(data: SyncQueueItem | SyncQueueItem[]) {
    // TODO: TMP ASYNC WAIT TO INIT DB
    await this.getDeffredDbApi();

    this.particlesResolver.enqueue(Array.isArray(data) ? data : [data]);
  }

  private onMessage(msg: MessageEvent<QueueChannelMessage>) {
    const { type, data } = msg.data;
    if (type === 'link') {
      this.saveLinks(data);
    } else if (type === 'particle') {
      this.saveParticles(data);
    } else if (type === 'sync') {
      this.enquueSync(data);
    }
  }
}

export default BackendQueueChannelListener;
