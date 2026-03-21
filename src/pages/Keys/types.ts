import type { ConnectMethod } from './ActionBar/types';

export const KEY_TYPE: Record<string, ConnectMethod | 'secrets'> = {
  keplr: 'keplr',
  readOnly: 'read-only',
  secrets: 'secrets',
  wallet: 'wallet',
};

export const KEY_LIST_TYPE = {
  key: 'key',
  secret: 'secret',
};
