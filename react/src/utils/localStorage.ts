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

function getObfuscationKey(): string {
  const KEY = 'cyb:device-key';
  let key = localStorage.getItem(KEY);
  if (!key) {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    key = btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(''));
    localStorage.setItem(KEY, key);
  }
  return key;
}

function obfuscate(json: string): string {
  const key = atob(getObfuscationKey());
  const result = new Uint8Array(json.length);
  for (let i = 0; i < json.length; i++) {
    result[i] = json.charCodeAt(i) ^ key.charCodeAt(i % key.length);
  }
  return btoa(String.fromCharCode(...result));
}

function deobfuscate(encoded: string): string {
  const key = atob(getObfuscationKey());
  const bytes = atob(encoded);
  const result: string[] = [];
  for (let i = 0; i < bytes.length; i++) {
    result.push(String.fromCharCode(bytes.charCodeAt(i) ^ key.charCodeAt(i % key.length)));
  }
  return result.join('');
}

const saveJsonToLocalStorage = (storageKey: JsonTypeKeys, data: JsonRecord) => {
  const json = JSON.stringify(data);
  if (storageKey === 'secrets') {
    localStorage.setItem(jsonKeyMap[storageKey], obfuscate(json));
  } else {
    localStorage.setItem(jsonKeyMap[storageKey], json);
  }
};

const loadJsonFromLocalStorage = (storageKey: JsonTypeKeys, defaultData: JsonRecord = {}) => {
  const raw = localStorage.getItem(jsonKeyMap[storageKey]);
  if (!raw) return defaultData;

  if (storageKey === 'secrets') {
    try {
      // Try deobfuscating first (new format)
      return JSON.parse(deobfuscate(raw));
    } catch {
      // Fallback: legacy plaintext JSON — migrate on next save
      try {
        return JSON.parse(raw);
      } catch {
        return defaultData;
      }
    }
  }

  return JSON.parse(raw);
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
