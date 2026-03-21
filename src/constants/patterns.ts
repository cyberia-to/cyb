import { BECH32_PREFIX, BECH32_PREFIX_VALOPER } from './config';

export const PATTERN_CYBER = new RegExp(`^${BECH32_PREFIX}[a-zA-Z0-9]{39}$`);

export const PATTERN_SPACE_PUSSY = /^pussy[a-zA-Z0-9]{39}$/;

export const PATTERN_IPFS_HASH = /^Qm[a-zA-Z0-9]{44}$/;

export const PATTERN_CYBER_CONTRACT = new RegExp(`^${BECH32_PREFIX}[a-zA-Z0-9]{59}$`);

export const PATTERN_CYBER_VALOPER = new RegExp(
  `^${BECH32_PREFIX_VALOPER}valoper[a-zA-Z0-9]{39}$`
);

export const PATTERN_COSMOS = /^cosmos[a-zA-Z0-9]{39}$/;

export const PATTERN_OSMOS = /^osmo[a-zA-Z0-9]{39}$/;

export const PATTERN_TERRA = /^terra[a-zA-Z0-9]{39}$/;

export const PATTERN_ETH = /^0x[a-fA-F0-9]{40}$/;

export const PATTERN_TX = /[0-9a-fA-F]{64}$/;

export const PATTERN_BLOCK = /^[0-9]+$/;

export const PATTERN_HTTP = /^https:\/\/|^http:\/\//;

export const PATTERN_HTML = /<\/?[\w\d]+>/i;
