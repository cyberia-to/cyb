import { ScriptEntrypointNames } from 'src/services/scripting/types';
import { KeyValueString } from 'src/types/data';

type JsonTypeKeys = 'secrets';

type StringTypeKeys =
  | ScriptEntrypointNames
  | `${ScriptEntrypointNames[0]}_enabled`
  | `${ScriptEntrypointNames[1]}_enabled`;

export type JsonRecord = Record<string, string | number | boolean>;

const jsonKeyMap: Record<JsonTypeKeys, string> = {
  secrets: 'secrets',
};

const stringKeyMap: Record<StringTypeKeys, string> = {
  particle: 'script_particle',
  // myParticle: 'script_particle_inference',
};

const keyValuesToObject = (data: KeyValueString[]) => {
  return Object.fromEntries(
    Object.values(data)
      .filter((row) => !!row?.key)
      .map((row) => [row.key, row.value])
  );
};

const saveJsonToLocalStorage = (storageKey: JsonTypeKeys, data: JsonRecord) => {
  localStorage.setItem(jsonKeyMap[storageKey], JSON.stringify(data));
};

const loadJsonFromLocalStorage = (storageKey: JsonTypeKeys, defaultData: JsonRecord = {}) => {
  const raw = localStorage.getItem(jsonKeyMap[storageKey]);
  return raw ? JSON.parse(raw) : defaultData;
};

const loadStringFromLocalStorage = (name: StringTypeKeys, defaultValue?: string) => {
  const keyName = stringKeyMap[name] || name;
  const result = localStorage.getItem(keyName) || defaultValue;
  return result;
};

const saveStringToLocalStorage = (name: StringTypeKeys, value: string) => {
  const keyName = stringKeyMap[name] || name;
  localStorage.setItem(keyName, value);
};

const getEntrypointKeyName = (name: ScriptEntrypointNames, prefix: 'enabled'): StringTypeKeys =>
  `${name}_${prefix}`;

export {
  saveJsonToLocalStorage,
  loadJsonFromLocalStorage,
  loadStringFromLocalStorage,
  saveStringToLocalStorage,
  getEntrypointKeyName,
  keyValuesToObject,
};
