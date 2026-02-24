import initAsync, { compile } from 'cyb-rune-wasm';
import { mapObjIndexed } from 'ramda';
import { BehaviorSubject, combineLatest, distinctUntilChanged, map, ReplaySubject } from 'rxjs';
import defaultParticleScript from 'src/services/scripting/rune/default/particle.rn';
import runtimeScript from 'src/services/scripting/rune/runtime.rn';
import { TabularKeyValues } from 'src/types/data';
import { entityToDto } from 'src/utils/dto';
import { keyValuesToObject } from 'src/utils/localStorage';
import { removeBrokenUnicode } from 'src/utils/string';
import { v4 as uuidv4 } from 'uuid';
import { extractRuneScript } from './helpers';
import {
  EngineContext,
  EntrypointParams,
  ScriptCallback,
  ScriptContext,
  // ScriptMyParticleParams,
  ScriptEntrypoints,
  ScriptMyCampanion,
  ScriptParticleParams,
  ScriptParticleResult,
} from './types';

type RuneEntrypoint = {
  readOnly: boolean;
  execute: boolean;
  funcName: string;
  funcParams: any[];
  params: Object; // context data
  input: string; // main code
  script: string; // runtime code
};

type RuneRunResult = {
  output: string;
  diagnosticsOutput: string;
  diagnostics: any[];
  error: string;
  result: any;
};

const compileConfig = {
  budget: 1_000_000,
  experimental: false,
  instructions: false,
  options: [],
};

const defaultRuneEntrypoint: RuneEntrypoint = {
  readOnly: false,
  execute: true,
  funcName: 'main',
  funcParams: {},
  params: {},
  input: defaultParticleScript,
  script: runtimeScript,
};

const _toRecord = (item: TabularKeyValues) => keyValuesToObject(Object.values(item));

export type LoadParams = {
  entrypoints: ScriptEntrypoints;
  secrets: TabularKeyValues;
};

// eslint-disable-next-line import/prefer-default-export
function enigine() {
  let entrypoints: Partial<ScriptEntrypoints> = {};
  let context: EngineContext = { params: {}, user: {}, secrets: {} };
  const isInitialized$ = new BehaviorSubject<boolean>(false);
  const entrypoints$ = new BehaviorSubject<Partial<ScriptEntrypoints>>({});

  const scriptCallbacks = new Map<string, ScriptCallback>();

  const isSoulInitialized$ = new ReplaySubject(1);
  combineLatest([isInitialized$, entrypoints$])
    .pipe(
      map(([isInitialized, entrypoints]) => !!(isInitialized && entrypoints.particle)),
      distinctUntilChanged()
    )
    .subscribe((v) => {
      isSoulInitialized$.next(v);
    });

  entrypoints$.subscribe((v) => {
    entrypoints = v;
  });

  const init = async () => {
    console.time('🔋 rune initialized');
    await initAsync();
    // window.rune = rune; // debug
    console.timeEnd('🔋 rune initialized');
    isInitialized$.next(true);
  };

  const pushContext = <K extends keyof ScriptContext>(
    name: K,
    value: ScriptContext[K] //| TabularKeyValues
  ) => {
    // context[name] =  toRecord(value as TabularKeyValues);
    context[name] = value;
  };

  const popContext = (names: (keyof ScriptContext)[]) => {
    const newContext = context;
    names.forEach((name) => {
      newContext[name] = {};
    });
    context = newContext;
  };

  const setEntrypoints = (scriptEntrypoints: ScriptEntrypoints) => {
    entrypoints = mapObjIndexed(
      (v) => ({ ...v, script: extractRuneScript(v.script) }),
      scriptEntrypoints
    );
    entrypoints$.next(entrypoints);
  };

  const run = async (
    script: string,
    compileParams: Partial<RuneEntrypoint>,
    callback?: ScriptCallback,
    scripts: Record<string, string> = {}
  ) => {
    const refId = uuidv4().toString();

    callback && scriptCallbacks.set(refId, callback);
    const scriptParams = {
      app: context,
      refId,
    };
    const compilerParams = {
      ...defaultRuneEntrypoint,
      ...compileParams,
      input: script,
      scripts: { ...scripts, runtime: runtimeScript },
      params: scriptParams,
    };
    // console.log('-----run', scriptParams);
    const outputData = await compile(compilerParams, compileConfig);

    // Parse the JSON string
    const { result, error } = outputData;

    try {
      scriptCallbacks.delete(refId);

      return {
        ...entityToDto(outputData),
        error,
        result: result
          ? result === '()'
            ? ''
            : JSON.parse(removeBrokenUnicode(result))
          : { action: 'error', message: 'No result' },
      } as RuneRunResult;
    } catch (e) {
      scriptCallbacks.delete(refId);

      console.log(`engine.run err ${compilerParams.funcName}`, e, outputData, compilerParams);
      return {
        diagnosticsOutput: `scripting engine error ${e}`,
        ...outputData,
        result: { action: 'error', message: e?.toString() || 'Unknown error' },
      } as RuneRunResult;
    }
  };

  const getParticleScriptOrAction = (): ['error' | 'pass' | 'script', string] => {
    if (!entrypoints.particle) {
      return ['error', ''];
    }

    const { script, enabled } = entrypoints.particle;

    if (!enabled) {
      return ['pass', ''];
    }

    return ['script', script];
  };

  const personalProcessor = async (params: ScriptParticleParams): Promise<ScriptParticleResult> => {
    const [resultType, script] = getParticleScriptOrAction();

    if (resultType === 'error') {
      return { action: 'error', message: 'No particle entrypoint' };
    }

    if (resultType !== 'script') {
      return { action: 'pass' };
    }

    const { cid, contentType, content } = params;
    const output = await run(script, {
      funcName: 'personal_processor',
      funcParams: [cid, contentType, content], //params as EntrypointParams,
    });
    const { action, content: outputContent } = output.result;

    if (action === 'error') {
      console.error(`RUNE: personalProcessor error: ${params.cid}`, params, output);
    }

    if (outputContent) {
      return { ...output.result, content: outputContent };
    }

    return output.result;
  };

  const executeFunction = async (
    script: string,
    funcName: string,
    funcParams: EntrypointParams
  ): Promise<ScriptParticleResult> => {
    console.log('-----executeFunction rune', funcName, funcParams);
    const output = await run(script, {
      funcName,
      funcParams,
      readOnly: true, // block to sign tx and add to ipfs
    });

    return output.result;
  };

  // const particleInference = async (
  //   userScript: string,
  //   funcParams: EntrypointParams
  // ): Promise<ScriptMyParticleResult> => {
  //   const output = await run(userScript, {
  //     funcName: 'particle_inference',
  //     funcParams,
  //   });

  //   return output.result;
  // };

  const askCompanion = async (
    cid: string,
    contentType: string,
    content: string,
    callback?: ScriptCallback
  ): Promise<ScriptMyCampanion> => {
    const [resultType, script] = getParticleScriptOrAction();
    if (resultType === 'error') {
      return {
        action: 'error',
        metaItems: [[{ type: 'text', text: 'No particle entrypoint' }]],
      };
    }

    if (resultType === 'pass') {
      return { action: 'pass', metaItems: [] };
    }

    const output = await run(
      script,
      {
        funcName: 'ask_companion',
        funcParams: [cid, contentType, content],
      },
      callback
    );

    if (output.result.action === 'error') {
      console.error('---askCompanion error', output);
      return {
        action: 'error',
        metaItems: [[{ type: 'text', text: output.error }]],
      };
    }

    return { action: 'answer', metaItems: output.result.content };
  };

  const executeCallback = async (refId: string, data: any) => {
    const callback = scriptCallbacks.get(refId);

    if (callback) {
      await callback(data);
    }
  };

  return {
    init,
    isSoulInitialized$,
    run,
    // particleInference,
    askCompanion,
    personalProcessor,
    setEntrypoints,
    pushContext,
    popContext,
    executeFunction,
    executeCallback,
    getDebug: () => ({
      context,
      entrypoints,
    }),
  };
}

const scriptEngine = enigine();

export type RuneEngine = typeof scriptEngine;

export default scriptEngine;
