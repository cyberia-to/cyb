import { env, FeatureExtractionPipeline, PipelineType, pipeline } from '@xenova/transformers';
import { proxy } from 'comlink';
import { combineLatest, Observable, ReplaySubject, Subject } from 'rxjs';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';

env.allowLocalModels = false;

type MlModelParams = {
  name: PipelineType;
  model: string;
};
const mlModelMap: Record<string, MlModelParams> = {
  featureExtractor: {
    name: 'feature-extraction',
    model: 'Xenova/all-MiniLM-L6-v2',
  },
  // summarization: {
  //   name: 'summarization',
  //   model: 'ahmedaeb/distilbart-cnn-6-6-optimised',
  // },
  // qa: {
  //   name: 'question-answering',
  //   model: 'Xenova/distilbert-base-uncased-distilled-squad',
  // },
};

const loadPipeline = (name: PipelineType, model: string, onProgress: (data: any) => void) => {
  return pipeline(name, model, {
    progress_callback: (progressData: any) => {
      try {
        const {
          status,
          progress,
          // name: modelName,
          loaded,
          total,
        } = progressData;

        const message = loaded ? `${model} - ${loaded}/${total} bytes` : model;

        const done = ['ready', 'error'].some((s) => s === status);

        const progrssStateItem = {
          status,
          message,
          done,
          progress: progress ? Math.round(progress) : 0,
        };
        // console.log('progress_callback', name, progressData);

        onProgress(progrssStateItem);
      } catch (e) {
        console.log('-------progresss error', model, e.toString());
      }
    },
  });
};

export type EmbeddingApi = {
  createEmbedding: (text: string) => Promise<number[]>;
  searchByEmbedding: (
    text: string,
    count?: number
  ) => ReturnType<DbApiWrapper['searchByEmbedding']>;
};

const createEmbeddingApi$ = (
  dbInstance$: Subject<DbApiWrapper>,
  featureExtractor$: Subject<FeatureExtractionPipeline>
) => {
  const replaySubject = new ReplaySubject(1);

  combineLatest([dbInstance$, featureExtractor$]).subscribe(([dbInstance, featureExtractor]) => {
    if (dbInstance && featureExtractor) {
      const createEmbedding = async (text: string) => {
        const output = await featureExtractor(text, {
          pooling: 'mean',
          normalize: true,
        });

        return output.data as number[];
      };

      const searchByEmbedding = async (text: string, count?: number) => {
        const vec = await createEmbedding(text);
        // console.log('----searchByEmbedding', vec);

        const rows = await dbInstance.searchByEmbedding(vec, count);
        //   console.log('----searcByEmbedding rows', rows);

        return rows;
      };

      const api = {
        createEmbedding,
        searchByEmbedding,
      };
      replaySubject.next(proxy(api));
    }
  });
  // .pipe(filter((v) => !!v))
  return replaySubject as Observable<EmbeddingApi>;
};

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export const createMlApi = (
  dbInstance$: Subject<DbApiWrapper>,
  broadcastApi: BroadcastChannelSender
) => {
  const featureExtractor$ = new Subject<FeatureExtractionPipeline>();
  const embeddingApi$ = createEmbeddingApi$(dbInstance$, featureExtractor$);

  const initPipelineInstance = async (alias: keyof typeof mlModelMap) => {
    const { name, model } = mlModelMap[alias];

    const pipeline = await loadPipeline(name, model, (data) =>
      broadcastApi.postMlSyncEntryProgress(alias, data)
    );
    if (name === 'feature-extraction') {
      featureExtractor$.next(pipeline as FeatureExtractionPipeline);
    }
    console.log(`${alias} - loaded`);
  };

  const init = async () => {
    broadcastApi.postServiceStatus('ml', 'starting');
    console.time('🔋 ml initialized');

    return Promise.all([
      initPipelineInstance('featureExtractor'),
      // initMlInstance('summarization'),
      // initMlInstance('qa'),
    ])
      .then((result) => {
        setTimeout(() => broadcastApi.postServiceStatus('ml', 'started'), 0);
        console.timeEnd('🔋 ml initialized');

        return result;
      })
      .catch((e) => broadcastApi.postServiceStatus('ml', 'error', e.toString()));
  };

  init();

  return { embeddingApi$, init };
};
