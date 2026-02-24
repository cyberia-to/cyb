import { Observable } from 'rxjs';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';
import rune from 'src/services/scripting/engine';
import runeDeps, { RuneInnerDeps } from 'src/services/scripting/runeDeps';
import { EmbeddingApi } from './mlApi';

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export const createRuneApi = (
  embeddingApi$: Observable<EmbeddingApi>,
  dbInstance$: Observable<DbApiWrapper>,
  broadcastApi: BroadcastChannelSender
) => {
  const setInnerDeps = (deps: Partial<RuneInnerDeps>) => runeDeps.setInnerDeps(deps);

  embeddingApi$.subscribe((embeddingApi) => {
    setInnerDeps({ embeddingApi });
  });

  dbInstance$.subscribe((dbApi) => {
    setInnerDeps({ dbApi });
  });

  rune.isSoulInitialized$.subscribe((value) => {
    value
      ? setTimeout(() => broadcastApi.postServiceStatus('rune', 'started'), 0)
      : broadcastApi.postServiceStatus('rune', 'inactive');
  });

  const init = async () => {
    broadcastApi.postServiceStatus('rune', 'starting');

    await rune.init();
    setInnerDeps({ rune });
  };

  init();

  return { rune, setInnerDeps, abort: runeDeps.abort };
};
