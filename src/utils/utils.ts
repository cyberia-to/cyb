/* eslint-disable no-await-in-loop */

import { Sha256 } from '@cosmjs/crypto';
import { fromBase64, fromUtf8, toBech32 } from '@cosmjs/encoding';
import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { Key } from '@keplr-wallet/types';
import bech32 from 'bech32';
import BigNumber from 'bignumber.js';
import { BECH32_PREFIX, BECH32_PREFIX_VAL_CONS } from 'src/constants/config';
import { localStorageKeys } from 'src/constants/localStorageKeys';
import { Option } from 'src/types';
import { ObjKeyValue } from 'src/types/data';
import { AccountValue } from 'src/types/defaultAccount';
import { toHex } from 'src/utils/encoding';
import cyberBostrom from '../image/large-green.png';
import customNetwork from '../image/large-orange-circle.png';
import cyberSpace from '../image/large-purple-circle.png';
import { LEDGER } from './config';

const DEFAULT_DECIMAL_DIGITS = 3;
const DEFAULT_CURRENCY = 'GoL';

const roundNumber = (num, scale) => {
  if (!`${num}`.includes('e')) {
    return +`${Math.floor(`${num}e+${scale}`)}e-${scale}`;
  }
  const arr = `${num}`.split('e');
  let sig = '';
  if (+arr[1] + scale > 0) {
    sig = '+';
  }
  const i = `${+arr[0]}e${sig}${+arr[1] + scale}`;
  const j = Math.floor(i);
  const k = +`${j}e-${scale}`;
  return k;
};

function numberWithCommas(x) {
  const parts = x.split('.');
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
  return parts.join('.');
}

const formatNumber = (number: number | string, toFixed?: number): string => {
  let formatted = number;

  if (toFixed) {
    formatted = roundNumber(formatted, toFixed);
    formatted = formatted.toFixed(toFixed + 1);
  }

  if (typeof number === 'string') {
    return numberWithCommas(formatted);
  }

  return formatted
    .toLocaleString('en')
    .replace(/(\.\d{0,})0+$/, '$1')
    .replace(/,/g, ' ');
};

const PREFIXES = [
  {
    prefix: 'T',
    power: 10 ** 12,
  },
  {
    prefix: 'G',
    power: 10 ** 9,
  },
  {
    prefix: 'M',
    power: 10 ** 6,
  },
  {
    prefix: 'K',
    power: 10 ** 3,
  },
];

export function formatCurrency(
  value,
  currency = DEFAULT_CURRENCY,
  decimalDigits = DEFAULT_DECIMAL_DIGITS,
  prefixCustom = PREFIXES
) {
  const { prefix = '', power = 1 } = prefixCustom.find((obj) => value >= obj.power) || {};

  return `${roundNumber(
    Number(value) / power,
    decimalDigits
  )} ${prefix}${currency.toLocaleUpperCase()}`;
}

const getDecimal = (number, _toFixed) => {
  const nstring = number.toString();
  const narray = nstring.split('.');
  const result = narray.length > 1 ? narray[1] : '000';
  return result;
};

const asyncForEach = async (array, callback) => {
  for (let index = 0; index < array.length; index++) {
    await callback(array[index], index, array);
  }
};

const fromBech32 = (operatorAddr, prefix = BECH32_PREFIX) => {
  const address = bech32.decode(operatorAddr);
  return bech32.encode(prefix, address.words);
};

export const consensusPubkey = (pubKey: string) => {
  const ed25519PubkeyRaw = fromBase64(pubKey);
  const addressData = sha256(ed25519PubkeyRaw).slice(0, 20);
  return toBech32(BECH32_PREFIX_VAL_CONS, addressData);
};

const trimString = (address: string, first = 3, second = 8) => {
  if (address && address.length > 11) {
    return `${address.substring(0, first)}...${address.substring(address.length - second)}`;
  }
  if (address && address.length < 11) {
    return address;
  }
  return '';
};

const exponentialToDecimal = (exponential) => {
  let decimal = exponential.toString().toLowerCase();
  if (decimal.includes('e+')) {
    const exponentialSplitted = decimal.split('e+');
    let postfix = '';
    for (
      let i = 0;
      i <
      +exponentialSplitted[1] -
        (exponentialSplitted[0].includes('.') ? exponentialSplitted[0].split('.')[1].length : 0);
      i++
    ) {
      postfix += '0';
    }
    decimal = exponentialSplitted[0].replace('.', '') + postfix;
  }
  if (decimal.toLowerCase().includes('e-')) {
    const exponentialSplitted = decimal.split('e-');
    let prefix = '0.';
    for (let i = 0; i < +exponentialSplitted[1] - 1; i++) {
      prefix += '0';
    }
    decimal = prefix + exponentialSplitted[0].replace('.', '');
  }
  return decimal;
};

function dhm(t) {
  const cd = 24 * 60 * 60 * 1000;
  const ch = 60 * 60 * 1000;
  let d = Math.floor(t / cd);
  let h = Math.floor((t - d * cd) / ch);
  let m = Math.round((t - d * cd - h * ch) / 60000);
  const pad = (n, unit) => {
    return n < 10 ? `0${n}${unit}` : `${n}${unit}`;
  };
  if (m === 60) {
    h += 1;
    m = 0;
  }
  if (h === 24) {
    d += 1;
    h = 0;
  }
  return [`${d}d`, pad(h, 'h'), pad(m, 'm')].join(':');
}

const downloadObjectAsJson = (exportObj, exportName) => {
  const dataStr = `data:text/json;charset=utf-8,${encodeURIComponent(JSON.stringify(exportObj))}`;
  const downloadAnchorNode = document.createElement('a');

  downloadAnchorNode.setAttribute('href', dataStr);
  downloadAnchorNode.setAttribute('download', `${exportName}.json`);
  document.body.appendChild(downloadAnchorNode);
  downloadAnchorNode.click();
  downloadAnchorNode.remove();
};

const isMobileTablet = () => {
  let hasTouchScreen = false;
  if ('maxTouchPoints' in navigator) {
    hasTouchScreen = navigator.maxTouchPoints > 0;
  } else if ('msMaxTouchPoints' in navigator) {
    hasTouchScreen = navigator.msMaxTouchPoints > 0;
  } else {
    const mQ = window.matchMedia && matchMedia('(pointer:coarse)');
    if (mQ && mQ.media === '(pointer:coarse)') {
      hasTouchScreen = !!mQ.matches;
    } else if ('orientation' in window) {
      hasTouchScreen = true; // deprecated, but good fallback
    } else {
      // Only as a last resort, fall back to user agent sniffing
      const UA = navigator.userAgent;
      hasTouchScreen =
        /\b(BlackBerry|webOS|iPhone|IEMobile)\b/i.test(UA) ||
        /\b(Android|Windows Phone|iPad|iPod)\b/i.test(UA);
    }
  }
  return hasTouchScreen;
};

const coinDecimals = (number) => {
  return number * 10 ** -18;
};

const convertResources = (number) => {
  return Math.floor(number * 10 ** -3);
};

function timeSince(timeMS: number) {
  const seconds = Math.floor(timeMS / 1000);

  if (seconds === 0) {
    return 'now';
  }

  let interval = Math.floor(seconds / 31536000);

  if (interval > 1) {
    return `${interval} years`;
  }
  interval = Math.floor(seconds / 2592000);
  if (interval > 1) {
    return `${interval} months`;
  }
  interval = Math.floor(seconds / 86400);
  if (interval > 1) {
    return `${interval} days`;
  }
  interval = Math.floor(seconds / 3600);
  if (interval > 1) {
    return `${interval} hours`;
  }
  interval = Math.floor(seconds / 60);
  if (interval > 1) {
    return `${interval} min`;
  }
  return `${Math.floor(seconds)} sec`;
}

const reduceBalances = (data): ObjKeyValue => {
  try {
    let balances = {};
    if (Object.keys(data).length > 0) {
      balances = data.reduce(
        (obj, item) => ({
          ...obj,
          [item.denom]: parseFloat(item.amount),
        }),
        {}
      );
    }
    return balances;
  } catch (error) {
    console.log(`error reduceBalances`, error);
    return {};
  }
};

// example: oneLiner -> message.module=wasm&message.action=/cosmwasm.wasm.v1.MsgStoreCode&store_code.code_id=${codeId}
function makeTags(oneLiner) {
  return oneLiner.split('&').map((pair) => {
    if (pair.indexOf('=') === -1) {
      throw new Error('Parsing error: Equal sign missing');
    }
    const parts = pair.split('=');
    if (parts.length > 2) {
      throw new Error(
        'Parsing error: Multiple equal signs found. If you need escaping support, please create a PR.'
      );
    }
    const [key, value] = parts;
    if (!key) {
      throw new Error('Parsing error: Key must not be empty');
    }
    return { key, value };
  });
}

function parseMsgContract(msg) {
  const json = fromUtf8(msg);

  return JSON.parse(json);
}
const replaceSlash = (text) => text.replace(/\//g, '%2F');

const encodeSlash = (text) => text.replace(/%2F/g, '/');

const groupMsg = (ArrMsg, size = 2) => {
  const link = [];
  for (let i = 0; i < Math.ceil(ArrMsg.length / size); i += 1) {
    link[i] = ArrMsg.slice(i * size, i * size + size);
  }
  return link;
};

const selectNetworkImg = (network) => {
  switch (network) {
    case 'bostrom':
      return cyberBostrom;
    case 'space-pussy':
      return cyberSpace;

    default:
      return customNetwork;
  }
};

const sha256 = (data) => {
  return new Uint8Array(new Sha256().update(data).digest());
};

function getDenomHash(path, baseDenom) {
  const parts = path.split('/');
  parts.push(baseDenom);
  const newPath = parts.slice().join('/');
  return `ibc/${toHex(sha256(newPath)).toUpperCase()}`;
}

function convertAmount(rawAmount, precision) {
  return new BigNumber(rawAmount)
    .shiftedBy(-precision)
    .dp(precision, BigNumber.ROUND_FLOOR)
    .toNumber();
}

function convertAmountReverce(rawAmount, precision) {
  return new BigNumber(rawAmount)
    .shiftedBy(precision)
    .dp(precision, BigNumber.ROUND_FLOOR)
    .toNumber();
}

function getDisplayAmount(rawAmount: number | string, precision: number): number {
  return parseFloat(
    new BigNumber(rawAmount)
      .shiftedBy(-precision)
      .dp(precision, BigNumber.ROUND_FLOOR)
      .toFixed(precision > 0 ? 3 : 0, BigNumber.ROUND_FLOOR)
  );
}

function getDisplayAmountReverce(rawAmount, precision) {
  return new BigNumber(rawAmount)
    .shiftedBy(precision)
    .dp(precision, BigNumber.ROUND_FLOOR)
    .toFixed(precision > 0 ? 3 : 0, BigNumber.ROUND_FLOOR);
}

function isNative(denom) {
  if (denom?.includes('ibc')) {
    return false;
  }
  return true;
}

const findPoolDenomInArr = (baseDenom: string, dataPools: Pool[]): Option<Pool> => {
  const findObj = dataPools.find((item) => item.poolCoinDenom === baseDenom);
  return findObj;
};

// REFACTOR: Probably wrong timestamp
const getNowUtcTime = (): number => {
  const now = new Date();
  const utcTime = new Date(
    now.getUTCFullYear(),
    now.getUTCMonth(),
    now.getUTCDate(),
    now.getUTCHours(),
    now.getUTCMinutes(),
    now.getUTCSeconds()
  );

  return utcTime.getTime();
};

const accountsKeplr = (accounts: Key): AccountValue => {
  const { pubKey, bech32Address, name } = accounts;
  const pk = toHex(new Uint8Array(pubKey));

  return {
    bech32: bech32Address,
    keys: 'keplr',
    pk,
    path: LEDGER.HDPATH,
    name,
  };
};

export function covertUint8ArrayToString(data: Uint8Array): string {
  return new TextDecoder().decode(data);
}

export {
  formatNumber,
  asyncForEach,
  getDecimal,
  fromBech32,
  trimString,
  exponentialToDecimal,
  dhm,
  downloadObjectAsJson,
  isMobileTablet,
  coinDecimals,
  convertResources,
  timeSince,
  reduceBalances,
  makeTags,
  parseMsgContract,
  replaceSlash,
  encodeSlash,
  groupMsg,
  selectNetworkImg,
  getDenomHash,
  getDisplayAmount,
  getDisplayAmountReverce,
  convertAmount,
  convertAmountReverce,
  isNative,
  findPoolDenomInArr,
  getNowUtcTime,
  accountsKeplr,
};

export const getMnemonic = (bech32?: string): string | null => {
  if (bech32) {
    const perAddr = localStorage.getItem(`cyb:mnemonic:${bech32}`);
    if (perAddr) return perAddr;
  }
  return localStorage.getItem(localStorageKeys.signer.mnemonic);
};

export const setMnemonic = (mnemonic: string, bech32?: string) => {
  localStorage.setItem(localStorageKeys.signer.mnemonic, mnemonic);
  if (bech32) {
    localStorage.setItem(`cyb:mnemonic:${bech32}`, mnemonic);
  }
};
