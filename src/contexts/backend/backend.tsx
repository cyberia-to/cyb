import { proxy, Remote } from 'comlink';
import React, { useContext, useEffect, useMemo, useState } from 'react';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { RESET_SYNC_STATE_ACTION_NAME } from 'src/redux/reducers/backend';
import RxBroadcastChannelListener from 'src/services/backend/channels/RxBroadcastChannelListener';
import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';
import { backgroundWorkerInstance } from 'src/services/backend/workers/background/service';
import { BackgroundWorker } from 'src/services/backend/workers/background/worker';
import { cozoDbWorkerInstance } from 'src/services/backend/workers/db/service';
import { CozoDbWorker } from 'src/services/backend/workers/db/worker';

import { DB_NAME } from 'src/services/CozoDb/cozoDb';
import { getIpfsOpts } from 'src/services/ipfs/config';
// import BroadcastChannelListener from 'src/services/backend/channels/BroadcastChannelListener';

import { Observable } from 'rxjs';
import { EmbeddingApi } from 'src/services/backend/workers/background/api/mlApi';
import { RuneEngine } from 'src/services/scripting/engine';
import { Option } from 'src/types';
import { createSenseApi, SenseApi } from './services/senseApi';

const setupStoragePersistence = async () => {
  let isPersistedStorage: boolean;
  try {
    isPersistedStorage = await navigator.storage.persisted();
    if (!isPersistedStorage) {
      await navigator.permissions.query({ name: 'persistent-storage' });
      isPersistedStorage = true;
    }
  } catch (error) {
    console.log('[Backend] failed to get persistence status', error);
    isPersistedStorage = false;
  }
  const message = isPersistedStorage ? `🔰 storage is persistent` : `⚠️ storage is non-persitent`;

  console.log(message);

  return isPersistedStorage;
};

type BackendProviderContextType = {
  cozoDbRemote: Remote<CozoDbWorker> | null;
  senseApi: SenseApi;
  ipfsApi: Remote<BackgroundWorker['ipfsApi']> | null;
  dbApi: DbApiWrapper | null;
  ipfsError?: string | null;
  isIpfsInitialized: boolean;
  isDbInitialized: boolean;
  isSyncInitialized: boolean;
  isReady: boolean;
  embeddingApi$: Promise<Observable<EmbeddingApi>>;
  rune: Remote<RuneEngine>;
};

const valueContext = {
  cozoDbRemote: null,
  senseApi: null,
  isIpfsInitialized: false,
  isDbInitialized: false,
  isSyncInitialized: false,
  isReady: false,
  dbApi: null,
  ipfsApi: null,
};

const BackendContext = React.createContext<BackendProviderContextType>(valueContext);

export function useBackend() {
  return useContext(BackendContext);
}

window.cyb.db = {
  clear: () => indexedDB.deleteDatabase(DB_NAME),
};

function BackendProvider({ children }: { children: React.ReactNode }) {
  const dispatch = useAppDispatch();
  // const { defaultAccount } = useAppSelector((state) => state.pocket);

  const [ipfsError, setIpfsError] = useState(null);

  const isDbInitialized = useAppSelector((state) => state.backend.services.db.status === 'started');
  const isIpfsInitialized = useAppSelector(
    (state) => state.backend.services.ipfs.status === 'started'
  );
  const isSyncInitialized = useAppSelector(
    (state) => state.backend.services.sync.status === 'started'
  );
  const [needPFSInitialize, setNeedPFSInitialize] = useState(false);

  const myAddress = useAppSelector(selectCurrentAddress);

  const { friends, following } = useAppSelector((state) => state.backend.community);

  // // TODO: preload from DB
  const followings = useMemo(() => {
    return Array.from(new Set([...friends, ...following]));
  }, [friends, following]);

  const isReady = isDbInitialized &&
      isIpfsInitialized &&
      isSyncInitialized &&
      !needPFSInitialize;
  const [_embeddingApi$, setEmbeddingApi] = useState<Option<Observable<EmbeddingApi>>>(undefined);
  // const embeddingApiRef = useRef<Observable<EmbeddingApi>>();
  useEffect(() => {
    console.log(
      process.env.IS_DEV
        ? '🧪 Starting backend in DEV mode...'
        : '🧬 Starting backend in PROD mode...'
    );

    (async () => {
      // embeddingApiRef.current = await backgroundWorkerInstance.embeddingApi$;
      const embeddingApiInstance$ = await backgroundWorkerInstance.embeddingApi$;
      setEmbeddingApi(embeddingApiInstance$);
    })();

    setupStoragePersistence();

    const _channel = new RxBroadcastChannelListener(dispatch);

    const startIpfsWithRetry = async (retries = 3, delays = [5000, 15000, 45000]) => {
      for (let attempt = 0; attempt <= retries; attempt++) {
        try {
          const ipfsOpts = getIpfsOpts();
          // eslint-disable-next-line no-await-in-loop
          await backgroundWorkerInstance.ipfsApi.start(ipfsOpts);
          setIpfsError(null);
          if (attempt > 0) {
            console.log(`🔰 IPFS connected after ${attempt + 1} attempts`);
          }
          return;
        } catch (err) {
          if (attempt < retries) {
            const delay = delays[attempt] || delays[delays.length - 1];
            console.warn(
              `⚠️ IPFS connection failed (attempt ${attempt + 1}/${retries + 1}), retrying in ${delay / 1000}s...`
            );
            // eslint-disable-next-line no-await-in-loop
            await new Promise((resolve) => {
              setTimeout(resolve, delay);
            });
          } else {
            setIpfsError(err);
            console.error(`☠️ IPFS failed after ${retries + 1} attempts: ${err}`);
          }
        }
      }
    };

    startIpfsWithRetry();

    cozoDbWorkerInstance.init().then(() => {
      // const dbApi = createDbApi();
      const dbApi = new DbApiWrapper();

      dbApi.init(proxy(cozoDbWorkerInstance));
      setDbApi(dbApi);
      // pass dbApi into background worker
      return backgroundWorkerInstance.injectDb(proxy(dbApi));
    });
  }, [dispatch]);

  useEffect(() => {
    backgroundWorkerInstance.setParams({ myAddress });
    dispatch({ type: RESET_SYNC_STATE_ACTION_NAME });
  }, [myAddress, dispatch]);

  useEffect(() => {
    isReady && console.log('🟢 backend started!');
  }, [isReady]);


  const [dbApi, setDbApi] = useState<DbApiWrapper | null>(null);

  const senseApi = useMemo(() => {
    if (isDbInitialized && dbApi && myAddress) {
      return createSenseApi(dbApi, myAddress, followings);
    }
    return null;
  }, [isDbInitialized, dbApi, myAddress, followings]);

  useEffect(() => {
    (async () => {
      backgroundWorkerInstance.setRuneDeps({
        address: myAddress,
        // TODO: proxify particular methods
        // senseApi: senseApi ? proxy(senseApi) : undefined,
        // signingClient: signingClient ? proxy(signingClient) : undefined,
      });
    })();
  }, [myAddress]);

  const ipfsApi = useMemo(() => backgroundWorkerInstance.ipfsApi, []);

  const valueMemo = useMemo(
    () =>
      ({
        rune: backgroundWorkerInstance.rune,
        embeddingApi$: backgroundWorkerInstance.embeddingApi$,
        cozoDbRemote: cozoDbWorkerInstance,
        ipfsApi,
        dbApi,
        senseApi,
        ipfsError,
        isIpfsInitialized,
        isDbInitialized,
        isSyncInitialized,
        isReady,
      }) as BackendProviderContextType,
    [
      isReady,
      isIpfsInitialized,
      isDbInitialized,
      isSyncInitialized,
      ipfsError,
      senseApi,
      dbApi,
      ipfsApi,
    ]
  );

  return <BackendContext.Provider value={valueMemo}>{children}</BackendContext.Provider>;
}

export default BackendProvider;
