import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit';
import { RouteItemT } from '../ui/type';

type HistoryItem = {
  time: string;
  action: {
    type: 'route';
    value: RouteItemT;
  };
};

const keyLS = 'timeHistory';

type SliceState = {
  [keyLS]: HistoryItem[];
};

function getItem() {
  const poolsGetItem = localStorage.getItem(keyLS);

  return poolsGetItem ? JSON.parse(poolsGetItem) : [];
}

const initialState: SliceState = {
  [keyLS]: getItem(),
};

function saveToLocalStorage(state: SliceState) {
  localStorage.setItem(keyLS, JSON.stringify(state[keyLS]));
}

const LIMIT_ARRAY = 35;

const slice = createSlice({
  name: 'timeHistory',
  initialState,
  reducers: {
    setTimeHistory: (state, { payload }: PayloadAction<HistoryItem['action']>) => {
      const timePayload = new Date().toISOString();

      const stateItem: HistoryItem = {
        time: timePayload,
        action: payload,
      };

      const newState = [...state[keyLS], stateItem];

      state[keyLS] =
        newState.length > LIMIT_ARRAY ? newState.slice(newState.length - LIMIT_ARRAY) : newState;

      saveToLocalStorage(state);
    },
  },
});

export const setTimeHistoryRoute = (url: string) => (dispatch: Dispatch) => {
  dispatch(
    setTimeHistory({
      type: 'route',
      value: {
        url,
      },
    })
  );
};

export const { setTimeHistory } = slice.actions;

export default slice.reducer;
