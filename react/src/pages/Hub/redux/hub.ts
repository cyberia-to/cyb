import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import { Option } from 'src/types';
import { ChannelList, NetworkList, TokenList } from 'src/types/hub';

enum Hub {
  channels = 'channels',
  tokens = 'tokens',
  networks = 'networks',
}

type SliceState = {
  [Hub.channels]: Option<ChannelList>;
  [Hub.tokens]: Option<TokenList>;
  [Hub.networks]: Option<NetworkList>;
};

const keyLS = (key: Hub) => `hub-${key}`;

function getItem(key: Hub) {
  const poolsGetItem = localStorage.getItem(keyLS(key));

  return poolsGetItem ? JSON.parse(poolsGetItem) : undefined;
}

function saveToLocalStorage(key: Hub, state: SliceState) {
  const data = state[key];

  localStorage.setItem(keyLS(key), JSON.stringify(data));
}

const initialState: SliceState = {
  [Hub.channels]: getItem(Hub.channels),
  [Hub.tokens]: getItem(Hub.tokens),
  [Hub.networks]: getItem(Hub.networks),
};

const slice = createSlice({
  name: 'hub',
  initialState,
  reducers: {
    setTokens: (state, { payload }: PayloadAction<TokenList>) => {
      state[Hub.tokens] = payload;

      saveToLocalStorage(Hub.tokens, state);
    },
    setNetworks: (state, { payload }: PayloadAction<NetworkList>) => {
      state[Hub.networks] = payload;

      saveToLocalStorage(Hub.networks, state);
    },
    setChannels: (state, { payload }: PayloadAction<ChannelList>) => {
      state[Hub.channels] = payload;

      saveToLocalStorage(Hub.channels, state);
    },
  },
});

export const { setTokens, setNetworks, setChannels } = slice.actions;

export default slice.reducer;
