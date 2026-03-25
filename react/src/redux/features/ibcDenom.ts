import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import { IbcDenomsArr } from 'src/types/ibc';

type SliceState = {
  denomTraces: undefined | IbcDenomsArr;
};

const localStoragePoolsKey = 'denomTraces';

const poolsGetItem = localStorage.getItem(localStoragePoolsKey);

const poolsInitState = poolsGetItem ? JSON.parse(poolsGetItem) : undefined;

const initialState: SliceState = {
  denomTraces: poolsInitState,
};

function saveToLocalStorage(state: SliceState) {
  const { denomTraces } = state;

  localStorage.setItem(localStoragePoolsKey, JSON.stringify(denomTraces));
}

const slice = createSlice({
  name: 'ibcDenom',
  initialState,
  reducers: {
    setDenomTraces: (state, { payload }: PayloadAction<IbcDenomsArr>) => {
      state.denomTraces = payload;

      saveToLocalStorage(state);
    },
  },
});

export const { setDenomTraces } = slice.actions;

export default slice.reducer;
