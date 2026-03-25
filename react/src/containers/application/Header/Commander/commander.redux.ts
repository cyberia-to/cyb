import { createSlice } from '@reduxjs/toolkit';

type SliceState = {
  value: string;
  isFocused: boolean;
};

const initialState: SliceState = {
  value: '',
  isFocused: false,
};

const slice = createSlice({
  name: 'commander',
  initialState,
  reducers: {
    setValue: (state, { payload }: { payload: string }) => {
      state.value = payload;
    },
    setFocus: (state, { payload }: { payload: boolean }) => {
      state.isFocused = payload;
    },
  },
});

export const { setValue, setFocus } = slice.actions;

export default slice.reducer;
