import { QueryLiquidityPoolsResponse } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/query';
import { createSlice, PayloadAction } from '@reduxjs/toolkit';

type SliceState = {
  pools: undefined | QueryLiquidityPoolsResponse;
};

const localStoragePoolsKey = 'liquidityPools';

const poolsGetItem = localStorage.getItem(localStoragePoolsKey);

const poolsInitState = poolsGetItem ? JSON.parse(poolsGetItem) : undefined;

const initialState: SliceState = {
  pools: poolsInitState,
};

function saveToLocalStorage(state: SliceState) {
  const { pools } = state;

  localStorage.setItem(localStoragePoolsKey, JSON.stringify(pools));
}

const slice = createSlice({
  name: 'warp',
  initialState,
  reducers: {
    setPools: (state, { payload }: PayloadAction<QueryLiquidityPoolsResponse>) => {
      state.pools = payload;

      saveToLocalStorage(state);
    },
  },
});

export const { setPools } = slice.actions;

export default slice.reducer;
