import { proxy, Remote } from 'comlink';
import React, { useContext, useEffect, useMemo, useState } from 'react';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import RxBroadcastChannelListener from 'src/services/backend/channels/RxBroadcastChannelListener';
import { backgroundWorkerInstance } from 'src/services/backend/workers/background/service';
import { cozoDbWorkerInstance } from 'src/services/backend/workers/db/service';

import { selectCurrentAddress } from 'src/redux/features/pocket';
import DbApiWrapper from 'src/services/backend/services/DbApi/DbApi';
import { BackgroundWorker } from 'src/services/backend/workers/background/worker';
import { CozoDbWorker } from 'src/services/backend/workers/db/worker';
import { getIpfsOpts } from 'src/services/ipfs/config';

import { RESET_SYNC_STATE_ACTION_NAME } from 'src/redux/reducers/backend';
import { DB_NAME } from 'src/services/CozoDb/cozoDb';
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

  console.log('[Backend] isPersistedStorage', isPersistedStorage);

  const message = isPersistedStorage
    ? `🔰 storage is persistent`
    : `⚠️ storage is non-persitent`;

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

const BackendContext =
  React.createContext<BackendProviderContextType>(valueContext);

export function useBackend() {
  return useContext(BackendContext);
}

window.cyb.db = {
  clear: () => indexedDB.deleteDatabase(DB_NAME),
};

const isMobileTauri =
  !!process.env.IS_TAURI &&
  /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);

function BackendProvider({ children }: { children: React.ReactNode }) {
  const dispatch = useAppDispatch();
  // const { defaultAccount } = useAppSelector((state) => state.pocket);

  const [ipfsError, setIpfsError] = useState(null);

  const isDbInitialized = useAppSelector(
    (state) => state.backend.services.db.status === 'started'
  );
  const isIpfsInitialized = useAppSelector(
    (state) => state.backend.services.ipfs.status === 'started'
  );
  const isSyncInitialized = useAppSelector(
    (state) => state.backend.services.sync.status === 'started'
  );
  const [needPFSInitialize, setNeedPFSInitialize] = useState(
    !!process.env.IS_TAURI && !isMobileTauri
  );

  const myAddress = useAppSelector(selectCurrentAddress);

  const { friends, following } = useAppSelector(
    (state) => state.backend.community
  );

  // // TODO: preload from DB
  const followings = useMemo(() => {
    return Array.from(new Set([...friends, ...following]));
  }, [friends, following]);

  // On mobile Tauri, skip heavy backend requirements for isReady
  const isReady = isMobileTauri
    ? true
    : isDbInitialized &&
      isIpfsInitialized &&
      isSyncInitialized &&
      !needPFSInitialize;
  const [embeddingApi$, setEmbeddingApi] =
    useState<Option<Observable<EmbeddingApi>>>(undefined);
  // const embeddingApiRef = useRef<Observable<EmbeddingApi>>();
  useEffect(() => {
    // On mobile Tauri, skip all heavy backend initialization
    // (IPFS, CozoDb, sync loops, ML embeddings, Rune engine)
    if (isMobileTauri) {
      console.log('[Backend] Mobile Tauri — skipping heavy backend services');
      return;
    }

    console.log(
      process.env.IS_DEV
        ? '🧪 Starting backend in DEV mode...'
        : '🧬 Starting backend in PROD mode...'
    );

    (async () => {
      // embeddingApiRef.current = await backgroundWorkerInstance.embeddingApi$;
      const embeddingApiInstance$ =
        await backgroundWorkerInstance.embeddingApi$;
      setEmbeddingApi(embeddingApiInstance$);
    })();

    setupStoragePersistence();

    const channel = new RxBroadcastChannelListener(dispatch);

    // In Tauri mode, IPFS connection is handled by the Tauri useEffect below
    // which waits for the daemon to be ready before connecting
    if (!process.env.IS_TAURI) {
      backgroundWorkerInstance.ipfsApi
        .start(getIpfsOpts())
        .then(() => {
          setIpfsError(null);
        })
        .catch((err) => {
          setIpfsError(err);
          console.log(`☠️ Ipfs error: ${err}`);
        });
    }

    cozoDbWorkerInstance.init().then(() => {
      // const dbApi = createDbApi();
      const dbApi = new DbApiWrapper();

      dbApi.init(proxy(cozoDbWorkerInstance));
      setDbApi(dbApi);
      // pass dbApi into background worker
      return backgroundWorkerInstance.injectDb(proxy(dbApi));
    });
  }, []);

  useEffect(() => {
    if (isMobileTauri) return;
    backgroundWorkerInstance.setParams({ myAddress });
    dispatch({ type: RESET_SYNC_STATE_ACTION_NAME });
  }, [myAddress, dispatch]);

  useEffect(() => {
    isReady && console.log('🟢 backend started!');
  }, [isReady]);

  useEffect(() => {
    if (process.env.IS_TAURI && !isMobileTauri) {
      console.log('[Backend] need initialize IPFS for TAURI env');
      (async () => {
        try {
          // Poll until IPFS daemon is running (started by Rust side)
          const maxAttempts = 30;
          for (let i = 0; i < maxAttempts; i++) {
            try {
              const response = await fetch('http://127.0.0.1:5001/api/v0/id', {
                method: 'POST',
              });
              if (response.ok) {
                console.log('[Backend] IPFS API is reachable');
                break;
              }
            } catch {
              // not ready yet
            }
            console.log(`[Backend] Waiting for IPFS API... (${i + 1}/${maxAttempts})`);
            await new Promise((r) => setTimeout(r, 2000));
          }

          setNeedPFSInitialize(false);
          console.log('[Backend] IPFS is ready for TAURI');

          // Reconnect the ipfs client now that daemon is up
          backgroundWorkerInstance.ipfsApi
            .start(getIpfsOpts())
            .then(() => {
              setIpfsError(null);
              console.log('[Backend] IPFS client reconnected');
            })
            .catch((err) => {
              setIpfsError(err);
              console.log(`[Backend] IPFS reconnect error: ${err}`);
            });
        } catch (error) {
          console.error('Failed to initialize IPFS for Tauri', error);
          setNeedPFSInitialize(false);
        }
      })();
    }
  }, []);

  const [dbApi, setDbApi] = useState<DbApiWrapper | null>(null);

  const senseApi = useMemo(() => {
    if (isDbInitialized && dbApi && myAddress) {
      return createSenseApi(dbApi, myAddress, followings);
    }
    return null;
  }, [isDbInitialized, dbApi, myAddress, followings]);

  useEffect(() => {
    if (isMobileTauri) return;
    (async () => {
      backgroundWorkerInstance.setRuneDeps({
        address: myAddress,
        // TODO: proxify particular methods
        // senseApi: senseApi ? proxy(senseApi) : undefined,
        // signingClient: signingClient ? proxy(signingClient) : undefined,
      });
    })();
  }, [myAddress]);

  const ipfsApi = useMemo(
    () => (isIpfsInitialized ? backgroundWorkerInstance.ipfsApi : null),
    [isIpfsInitialized]
  );

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
      } as BackendProviderContextType),
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

  return (
    <BackendContext.Provider value={valueMemo}>
      {children}
    </BackendContext.Provider>
  );
}

export default BackendProvider;
