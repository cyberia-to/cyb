import { createSlice } from '@reduxjs/toolkit';

const miningSlice = createSlice({
  name: 'mining',
  initialState: {
    active: false,
  },
  reducers: {
    setMiningActive(state, action: { payload: boolean }) {
      state.active = action.payload;
    },
  },
});

export const { setMiningActive } = miningSlice.actions;
export default miningSlice.reducer;
