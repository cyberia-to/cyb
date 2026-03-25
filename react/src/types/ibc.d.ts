export type IbcDenoms = {
  sourceChannelId: string;
  baseDenom: string;
  ibcDenom: string;
};

export type IbcDenomsArr = {
  [key: string]: IbcDenoms;
};

export type TracesDenomFuncResponse = {
  denom: string;
  coinDecimals: number;
  path: string;
  coinImageCid: string;
  native: boolean;
};

export type TracesDenomFuncType = (denom: string) => TracesDenomFuncResponse[];
