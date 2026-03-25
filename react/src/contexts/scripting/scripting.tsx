import { proxy, Remote } from 'comlink';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { selectCurrentPassport } from 'src/features/passport/passports.redux';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { selectRuneEntypoints, setEntrypoint } from 'src/redux/reducers/scripting';
import { EmbeddingApi } from 'src/services/backend/workers/background/api/mlApi';
import { RuneEngine } from 'src/services/scripting/engine';
import { UserContext } from 'src/services/scripting/types';
import { Option } from 'src/types';
import { useBackend } from '../backend/backend';

type RuneFrontend = Omit<RuneEngine, 'isSoulInitialized$'>;

type ScriptingContextType = {
  isSoulInitialized: boolean;
  rune: Option<Remote<RuneFrontend>>;
  embeddingApi: Option<EmbeddingApi>;
};

const ScriptingContext = React.createContext<ScriptingContextType>({
  isSoulInitialized: false,
  rune: undefined,
  embeddingApi: undefined,
});

export function useScripting() {
  return React.useContext(ScriptingContext);
}

function ScriptingProvider({ children }: { children: React.ReactNode }) {
  const { rune: runeBackend, ipfsApi, isIpfsInitialized, embeddingApi$ } = useBackend();

  const [isSoulInitialized, setIsSoulInitialized] = useState(false);
  const runeRef = useRef<Option<Remote<RuneFrontend>>>();
  const embeddingApiRef = useRef<Option<Remote<EmbeddingApi>>>();

  const dispatch = useAppDispatch();
  const runeEntryPoints = useAppSelector(selectRuneEntypoints);
  const citizenship = useAppSelector(selectCurrentPassport);
  const secrets = useAppSelector((state) => state.scripting.context.secrets);

  useEffect(() => {
    runeBackend.pushContext('secrets', secrets);

    const setupObservervable = async () => {
      const { isSoulInitialized$ } = runeBackend;

      const soulSubscription = (await isSoulInitialized$).subscribe((v) => {
        if (v) {
          runeRef.current = runeBackend;
          console.log('👻 soul initalized');
        }
        setIsSoulInitialized(!!v);
      });

      const embeddingApiSubscription = (await embeddingApi$).subscribe(
        proxy((embeddingApi) => {
          if (embeddingApi) {
            embeddingApiRef.current = embeddingApi;
            console.log('+ embedding api initalized', embeddingApi);
          }
        })
      );

      return () => {
        soulSubscription.unsubscribe();
        embeddingApiSubscription.unsubscribe();
      };
    };

    setupObservervable();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [embeddingApi$, runeBackend, secrets]);

  useEffect(() => {
    if (!isSoulInitialized || !runeRef.current) {
      return;
    }

    if (citizenship) {
      const particleCid = citizenship.extension.particle;

      runeRef.current.pushContext('user', {
        address: citizenship.owner,
        nickname: citizenship.extension.nickname,
        citizenship,
        particle: particleCid,
      } as UserContext);
    } else {
      runeRef.current.popContext(['user']);
    }
  }, [citizenship, isSoulInitialized]);

  useEffect(() => {
    if (isSoulInitialized && runeRef.current) {
      runeRef.current.pushContext('secrets', secrets);
    }
  }, [secrets, isSoulInitialized]);

  useEffect(() => {
    (async () => {
      if (citizenship && ipfsApi) {
        const particleCid = citizenship.extension.particle;

        if (particleCid && isIpfsInitialized) {
          (async () => {
            const result = await ipfsApi.fetchWithDetails(particleCid, 'text');

            dispatch(setEntrypoint({ name: 'particle', code: result?.content || '' }));
          })();
        }
      }
    })();
  }, [citizenship, isIpfsInitialized, ipfsApi, dispatch]);

  useEffect(() => {
    runeBackend.setEntrypoints(runeEntryPoints);
  }, [runeEntryPoints, runeBackend]);

  const value = useMemo(() => {
    return {
      rune: runeRef.current,
      embeddingApi: embeddingApiRef.current,
      isSoulInitialized,
    };
  }, [isSoulInitialized]);

  return <ScriptingContext.Provider value={value}>{children}</ScriptingContext.Provider>;
}

export default ScriptingProvider;
