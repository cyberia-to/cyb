import { createSlice } from '@reduxjs/toolkit';

type MiningStatus = {
  mining: boolean;
  hashrate: number;
  total_hashes: number;
  elapsed_secs: number;
  pending_proofs: number;
};

const miningSlice = createSlice({
  name: 'mining',
  initialState: {
    active: false,
    hashrate: 0,
    status: null as MiningStatus | null,
  },
  reducers: {
    setMiningActive(state, action: { payload: boolean }) {
      state.active = action.payload;
      if (!action.payload) {
        state.hashrate = 0;
        state.status = null;
      }
    },
    setMiningHashrate(state, action: { payload: number }) {
      state.hashrate = action.payload;
    },
    setMiningStatus(state, action: { payload: MiningStatus }) {
      if (action.payload.mining) {
        state.status = action.payload;
        state.active = true;
        state.hashrate = action.payload.hashrate;
      } else {
        state.status = null;
        state.active = false;
        state.hashrate = 0;
      }
    },
  },
});

export type { MiningStatus };
export const { setMiningActive, setMiningHashrate, setMiningStatus } = miningSlice.actions;
export default miningSlice.reducer;
