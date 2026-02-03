import { Coin } from '@cosmjs/stargate';
import { CoinDenom } from '@osmonauts/math/types';
import { Asset, Chain } from '@chain-registry/types';

export type Token = {
  logo?: string;
  denom: string;
  asset?: Asset;
  chain?: Chain;
  price?: number;
  symbol?: string;
  amount?: string;
  value?: string;
  $value?: string;
  balance?: Coin;
};

export type TokenList = Token[] & {
  rest: Token[];
  hash: Record<CoinDenom, Token>;
};
