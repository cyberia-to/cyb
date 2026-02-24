import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';

export type AssetsType = {
  [key: string]: number;
};

export interface PoolsWithAssetsType extends Pool {
  assets: AssetsType;
}

export interface PoolsWithAssetsCapType extends PoolsWithAssetsType {
  cap: number;
}

export type OptionNeverArray<T> = T | never[];

export type TypeTab = 'create-pool' | 'add-liquidity' | 'sub-liquidity';

export enum TypeTabEnum {
  ADD_LIQUIDITY = 'add-liquidity',
  CREATE_POOL = 'create-pool',
  SUB_LIQUIDITY = 'sub-liquidity',
}

export interface MyPoolsT extends Pool {
  coinDenom: string;
  myTokenAmount: number;
}
