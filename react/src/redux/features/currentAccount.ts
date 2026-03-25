import { createSlice, PayloadAction } from '@reduxjs/toolkit';

type CommunityEntryType = string[];

type SliceState = {
  community: {
    following: CommunityEntryType;
    followers: CommunityEntryType;
    friends: CommunityEntryType;
  };
};

const initialState: SliceState = {
  community: {
    following: [],
    followers: [],
    friends: [],
  },
};

const slice = createSlice({
  name: 'currentAccount',
  initialState,
  reducers: {
    setCommunity: (state, { payload }: PayloadAction<SliceState['community']>) => {
      state.community = payload;
    },
  },
});

export const { setCommunity } = slice.actions;

export default slice.reducer;
