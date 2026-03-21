import { PayloadAction, createSlice } from '@reduxjs/toolkit';
import { ActionBarStates } from 'src/containers/Search/constants';
import { AccountValue } from 'src/types/defaultAccount';

export interface LSAddress {
  address: string;
  keys: AccountValue['keys'];
}

interface ActionBarState {
  stage: ActionBarStates;
  toCid: string | null;
  fromCid: string | null;
  txHash?: string | null;
  txHeight: string | null;
  contentHash: string;

  address?: LSAddress;
  errorMessage?: string;
}

const initialState: ActionBarState = {
  stage: ActionBarStates.STAGE_INIT,
  txHeight: null,
  contentHash: '',
  fromCid: null,
  toCid: null,
};

const actionBarSlice = createSlice({
  name: 'actionBar',
  initialState,
  reducers: {
    setActionBarStage(state, { payload }: PayloadAction<ActionBarStates>) {
      state.stage = payload;
    },
    setToCid(state, { payload }: PayloadAction<string | null>) {
      state.toCid = payload;
    },
    setFromCid(state, { payload }: PayloadAction<string | null>) {
      state.fromCid = payload;
    },
    setTxHash(state, { payload }: PayloadAction<string | null>) {
      state.txHash = payload;
    },
    setTxHeight(state, { payload }: PayloadAction<string | null>) {
      state.txHeight = payload;
    },
    setContentHash(state, { payload }: PayloadAction<string>) {
      state.contentHash = payload;
    },
    setLSAddress(state, { payload }: PayloadAction<LSAddress | undefined>) {
      state.address = payload;
    },
    setErrorMessage(state, { payload }: PayloadAction<string | undefined>) {
      state.errorMessage = payload;
    },
    clearActionBarState() {
      return { ...initialState };
    },
  },
});

export const {
  setActionBarStage,
  setToCid,
  setFromCid,
  setTxHash,
  setTxHeight,
  setLSAddress,
  setContentHash,
  setErrorMessage,
  clearActionBarState,
} = actionBarSlice.actions;

export default actionBarSlice.reducer;
