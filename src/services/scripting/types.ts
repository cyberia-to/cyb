import { Citizenship } from 'src/types/citizenship';
import { KeyValueString } from 'src/types/data';
import { JsonRecord } from 'src/utils/localStorage';

type ParamsContext = {
  path?: string[];
  query?: string;
  search?: { [k: string]: string };
};

export type UserContext = {
  address?: string;
  nickname?: string;
  passport?: Citizenship;
  particle?: string | null;
};

type ScriptContext = {
  params: ParamsContext;
  user: UserContext;
  secrets: JsonRecord;
};

type EngineContext = Omit<ScriptContext, 'secrets'> & {
  secrets: KeyValueString;
};

type ScriptEntrypoint = {
  title: string;
  script: string;
  enabled: boolean;
};

type ScriptEntrypoints = {
  particle: ScriptEntrypoint;
  // myParticle: ScriptEntrypoint;
};

type ScriptEntrypointNames = keyof ScriptEntrypoints;

type ScriptCallback = (data: any) => Promise<void>;

type ScriptParticleParams = {
  cid?: string;
  contentType?: string;
  content?: string;
};

// type ScriptMyParticleParams = {
//   nickname?: string;
//   input?: string;
// };

type ScriptParticleResult = {
  action: 'pass' | 'cid_result' | 'content_result' | 'hide' | 'error';
  cid?: string;
  content?: string;
  message?: string;
};

type ScriptMyParticleResult = {
  action: 'pass' | 'answer' | 'error';
  answer?: string;
};

type MetaLinkComponent = {
  type: 'link';
  url: string;
  title: string;
};

type MetaTextComponent = {
  type: 'text';
  text: string;
};

type ScriptMyCampanion = {
  action: 'pass' | 'answer' | 'error';
  metaItems: (MetaLinkComponent | MetaTextComponent)[][];
};

// type ScriptScopeParams = {
//   particle?: ScriptParticleParams;
//   myParticle?: ScriptMyParticleParams;
//   refId?: string;
// };

type EntrypointParams = {
  [key: string]: any;
};

type ScriptExecutionResult = {
  error?: string;
  result: ScriptParticleResult;
  diagnosticsOutput?: string;
  output?: string;
  diagnostics?: object[];
  instructions?: string;
};

export type {
  ScriptContext,
  UserContext,
  ParamsContext,
  ScriptEntrypointNames,
  ScriptEntrypoint,
  ScriptCallbackStatus,
  ScriptCallback,
  EntrypointParams,
  ScriptParticleParams,
  // ScriptMyParticleParams,
  // ScriptScopeParams,
  ScriptExecutionResult,
  ScriptEntrypoints,
  ScriptMyParticleResult,
  ScriptParticleResult,
  EngineContext,
  ScriptMyCampanion,
};
