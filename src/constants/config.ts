import { Networks } from 'src/types/networks';
import safeLocalStorage from 'src/utils/safeLocalStorage';
import defaultNetworks from './defaultNetworks';

// FIXME: seems temp
export function isWorker() {
  return typeof WorkerGlobalScope !== 'undefined' && self instanceof WorkerGlobalScope;
}

const LOCALSTORAGE_CHAIN_ID = !isWorker() && safeLocalStorage.getItem('chainId');

const DEFAULT_CHAIN_ID: Networks.BOSTROM | Networks.SPACE_PUSSY =
  LOCALSTORAGE_CHAIN_ID || process.env.CHAIN_ID || Networks.BOSTROM;

export const CHAIN_ID = DEFAULT_CHAIN_ID;

export const LCD_URL = process.env.LCD_URL || defaultNetworks[DEFAULT_CHAIN_ID].LCD_URL;

export const RPC_URL = process.env.RPC_URL || defaultNetworks[DEFAULT_CHAIN_ID].RPC_URL;

export const WEBSOCKET_URL =
  process.env.WEBSOCKET_URL || defaultNetworks[DEFAULT_CHAIN_ID].WEBSOCKET_URL;

export const INDEX_HTTPS = process.env.INDEX_HTTPS || defaultNetworks[DEFAULT_CHAIN_ID].INDEX_HTTPS;

export const INDEX_WEBSOCKET =
  process.env.INDEX_WEBSOCKET || defaultNetworks[DEFAULT_CHAIN_ID].INDEX_WEBSOCKET;

export const BECH32_PREFIX =
  process.env.BECH32_PREFIX || defaultNetworks[DEFAULT_CHAIN_ID].BECH32_PREFIX;

const BECH32_PREFIX_VAL = `${BECH32_PREFIX}val`;

export const BECH32_PREFIX_VALOPER = `${BECH32_PREFIX_VAL}oper`;

export const BECH32_PREFIX_VAL_CONS = `${BECH32_PREFIX_VAL}cons`;

export const BASE_DENOM = process.env.BASE_DENOM || defaultNetworks[DEFAULT_CHAIN_ID].BASE_DENOM;

export const DENOM_LIQUID =
  process.env.DENOM_LIQUID || defaultNetworks[DEFAULT_CHAIN_ID].DENOM_LIQUID;

export const CYBER_GATEWAY = process.env.CYBER_GATEWAY || 'https://gateway.ipfs.cybernode.ai';

export const DIVISOR_CYBER_G = 10 ** 9;

export const DEFAULT_GAS_LIMITS = 200000;

export const COIN_DECIMALS_RESOURCE = 3;

export const { MEMO_KEPLR } = defaultNetworks[DEFAULT_CHAIN_ID];
