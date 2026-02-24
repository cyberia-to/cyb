import { CyberClient, SigningCyberClient } from '@cybercongress/cyber-js';
import { ProxyMarked } from 'comlink';
import { BehaviorSubject, first, Subject } from 'rxjs';
import { RPC_URL } from 'src/constants/config';
import { getPassportByNickname } from 'src/containers/portal/utils';
import { SenseApi } from 'src/contexts/backend/services/senseApi';
import { Option } from 'src/types';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { getSearchQuery, searchByHash } from 'src/utils/search/utils';
import DbApiWrapper from '../backend/services/DbApi/DbApi';
import { IpfsApi } from '../backend/workers/background/api/ipfsApi';
import { EmbeddingApi } from '../backend/workers/background/api/mlApi';
import { sendCyberlink } from '../neuron/neuronApi';
import { RuneEngine } from './engine';
import { extractRuneScript } from './helpers';

export type RuneInnerDeps = {
  ipfsApi: Option<IpfsApi>;
  rune: Option<RuneEngine>;
  queryClient: Option<CyberClient>;
  embeddingApi: Option<EmbeddingApi>;
  dbApi: Option<DbApiWrapper>;
  signingClient: Option<SigningCyberClient & ProxyMarked>;
  // signer?: Option<OfflineSigner>;
  senseApi: Option<SenseApi & ProxyMarked>;
  address: Option<NeuronAddress>;
};

type SubjectDeps<T> = {
  [K in keyof T]: BehaviorSubject<T[K]> | Subject<T[K]>;
};

const createRuneDeps = () => {
  const subjectDeps: SubjectDeps<RuneInnerDeps> = {
    // Initialize subjects for each dependency
    ipfsApi: new BehaviorSubject<RuneInnerDeps['ipfsApi']>(undefined),
    rune: new BehaviorSubject<RuneInnerDeps['rune']>(undefined),
    queryClient: new BehaviorSubject<RuneInnerDeps['queryClient']>(undefined),
    embeddingApi: new BehaviorSubject<Option<EmbeddingApi>>(undefined),
    signingClient: new BehaviorSubject<RuneInnerDeps['signingClient']>(undefined),
    senseApi: new BehaviorSubject<RuneInnerDeps['senseApi']>(undefined),
    address: new BehaviorSubject<RuneInnerDeps['address']>(undefined),
    dbApi: new BehaviorSubject<RuneInnerDeps['dbApi']>(undefined),
  };

  let abortController: Option<AbortController>;

  const defferedDependency = (name: keyof RuneInnerDeps): Promise<RuneInnerDeps[typeof name]> => {
    return new Promise((resolve) => {
      const item$ = subjectDeps[name] as BehaviorSubject<RuneInnerDeps[typeof name]>;
      if (item$.getValue()) {
        resolve(item$.getValue());
      }

      item$
        .pipe(
          first((value) => !!value) // Automatically unsubscribes after the first valid value
          // tap((v) => console.log('------defferedDependency', name, v))
        )
        .subscribe((value) => {
          resolve(value);
        });
    });
  };

  CyberClient.connect(RPC_URL).then((client) => {
    subjectDeps.queryClient?.next(client);
  });

  const setInnerDeps = (externalDeps: Partial<RuneInnerDeps>) => {
    Object.keys(externalDeps)
      .filter((name) => externalDeps[name as keyof RuneInnerDeps] !== undefined)
      .forEach((name) => {
        const item = externalDeps[name as keyof RuneInnerDeps];
        subjectDeps[name as keyof RuneInnerDeps].next(item);
      });
  };

  const graphSearch = async (query: string, page = 0) => {
    const queryClient = (await defferedDependency('queryClient')) as CyberClient;

    const keywordHash = await getSearchQuery(query);

    return searchByHash(queryClient, keywordHash, page);
  };

  const getIpfsTextConent = async (cid: string) => {
    const ipfsApi = (await defferedDependency('ipfsApi')) as IpfsApi;
    return ipfsApi.fetchWithDetails(cid, 'text');
  };

  const evalScriptFromIpfs = async (cid: ParticleCid, funcName: string, params = {}) => {
    try {
      const result = await getIpfsTextConent(cid);
      if (result?.content === undefined) {
        return { action: 'error', message: 'Particle not found' };
      }
      // in case of soul script is mixed with markdown
      // need to extract pure script
      const pureScript = extractRuneScript(result.content);

      const rune = (await defferedDependency('rune')) as RuneEngine;

      return rune.executeFunction(pureScript, funcName, params);
    } catch (e) {
      return { action: 'error', message: e.toString() };
    }
  };

  const executeScriptCallback = async (refId: string, data = {}) => {
    try {
      const rune = (await defferedDependency('rune')) as RuneEngine;
      return rune.executeCallback(refId, data);
    } catch (e) {
      return { action: 'error', message: e.toString() };
    }
  };

  const createAbortController = () => {
    abortController = new AbortController();
    return abortController;
  };

  const abort = () => {
    abortController?.abort();
  };

  const cybApi = {
    createAbortController,
    graphSearch,
    cyberlink: async (from: string, to: string) => {
      const address = subjectDeps.address.getValue();
      if (!address) {
        throw new Error('Connect your wallet first');
      }
      const senseApi = (await defferedDependency('senseApi')) as SenseApi;
      const signingClient = (await defferedDependency('signingClient')) as SigningCyberClient;

      return sendCyberlink(address, from, to, {
        senseApi,
        signingClient,
      });
    },
    getPassportByNickname: async (nickname: string) => {
      const queryClient = await defferedDependency('queryClient');
      const passport = await getPassportByNickname(queryClient, nickname);

      return passport;
    },
    searcByEmbedding: async (text: string, count = 10) => {
      const embeddingApi = (await defferedDependency('embeddingApi')) as EmbeddingApi;
      await defferedDependency('dbApi');
      // console.log('----searcByEmbedding', text);
      return embeddingApi.searchByEmbedding(text, count);
    },
    evalScriptFromIpfs,
    getIpfsTextConent,
    addContenToIpfs: async (content: string) => {
      const ipfsApi = (await defferedDependency('ipfsApi')) as IpfsApi;

      return ipfsApi.addContent(content);
    },
    executeScriptCallback,
  };

  return { setInnerDeps, cybApi, abort };
};

const runeDeps = createRuneDeps();

export type RuneDeps = typeof runeDeps;

// export type EngineDeps = ReturnType<typeof createRuneDeps>;

export default runeDeps;
