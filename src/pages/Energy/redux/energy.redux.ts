import { PayloadAction, createSlice } from '@reduxjs/toolkit';
import { Coin } from '@cosmjs/stargate';
import { StatusTx } from 'src/features/ibc-history/HistoriesItem';
import { StatusOrder } from './utils';

type SliceState = {
  statusOrder: StatusOrder;
  selectPlan?: { keyPackage: string; tokenIn: Coin };
  tokenSell: 'OSMO' | 'ATOM';
  swapResult?: {
    swapTx: string;
    tokens: Coin[];
  };
  ibcResult?: {
    ibcHash: string;
    status: StatusTx;
  };
};

const initStateTokenSell = 'OSMO';

const keyEnergyStateApp = 'energy-state-app';

const stateGetItem = localStorage.getItem(keyEnergyStateApp);

const poolsInitState = stateGetItem
  ? (JSON.parse(stateGetItem) as SliceState)
  : undefined;

const initialState: SliceState = {
  statusOrder: poolsInitState
    ? poolsInitState.statusOrder
    : StatusOrder.SELECT_PACK,
  selectPlan: poolsInitState?.selectPlan,
  tokenSell: poolsInitState ? poolsInitState.tokenSell : initStateTokenSell,
  swapResult: poolsInitState?.swapResult,
  ibcResult: poolsInitState?.ibcResult,
};

function saveToLocalStorage(state: SliceState) {
  localStorage.setItem(keyEnergyStateApp, JSON.stringify(state));
}

const slice = createSlice({
  name: 'energyPackages',
  initialState,
  reducers: {
    setStatusOrder: (state, { payload }: PayloadAction<StatusOrder>) => {
      state.statusOrder = payload;
      saveToLocalStorage(state);
    },
    setSelectPlan: (
      state,
      { payload }: PayloadAction<SliceState['selectPlan']>
    ) => {
      state.selectPlan = payload;
      saveToLocalStorage(state);
    },
    setTokenSell: (
      state,
      { payload }: PayloadAction<SliceState['tokenSell']>
    ) => {
      state.tokenSell = payload;
      saveToLocalStorage(state);
    },
    setSwapResult: (
      state,
      { payload }: PayloadAction<SliceState['swapResult']>
    ) => {
      state.swapResult = payload;
      saveToLocalStorage(state);
    },
    setIbcResult: (
      state,
      { payload }: PayloadAction<SliceState['ibcResult']>
    ) => {
      state.ibcResult = payload;
      saveToLocalStorage(state);
    },
    resetEnergy: (state) => {
      state.selectPlan = undefined;
      state.swapResult = undefined;
      state.ibcResult = undefined;
      state.statusOrder = StatusOrder.SELECT_PACK;

      localStorage.removeItem(keyEnergyStateApp);
    },
  },
});

export const {
  setStatusOrder,
  setSelectPlan,
  setSwapResult,
  setTokenSell,
  setIbcResult,
  resetEnergy,
} = slice.actions;

export default slice.reducer;
