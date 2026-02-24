/* eslint-disable camelcase */
import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import type { SliceActions } from 'src/redux/types';
import defaultParticleScript from 'src/services/scripting/rune/default/particle.rn';
import {
  ScriptContext,
  ScriptEntrypointNames,
  ScriptEntrypoints,
} from 'src/services/scripting/types';
import {
  getEntrypointKeyName,
  loadJsonFromLocalStorage,
  loadStringFromLocalStorage,
  saveJsonToLocalStorage,
  saveStringToLocalStorage,
} from 'src/utils/localStorage';
import { RootState } from '../store';

type SliceState = {
  context: ScriptContext; // Omit<ScriptContext, 'secrets' | 'user'>;
  scripts: {
    entrypoints: ScriptEntrypoints;
  };
};

const particleEnabled = loadStringFromLocalStorage(
  getEntrypointKeyName('particle', 'enabled'),
  undefined
);

const isParticleScriptEnabled = particleEnabled !== undefined ? !!particleEnabled : true;

console.log('----isParticleScriptEnabled', isParticleScriptEnabled);

const initialScriptEntrypoints: ScriptEntrypoints = {
  particle: {
    title: 'Personal processor',
    script: loadStringFromLocalStorage('particle', defaultParticleScript) as string,
    enabled: !!isParticleScriptEnabled,
  },
  // myParticle: {
  //   title: 'Social inference',
  //   script: '',
  //   enabled: true,
  // },
};

const initialState: SliceState = {
  context: {
    secrets: loadJsonFromLocalStorage('secrets', {}),
    params: {},
    user: {},
  },
  scripts: {
    entrypoints: initialScriptEntrypoints,
  },
};

// export function setSecrets(secrets: TabularKeyValues): AppThunk {
//   return (dispatch) => {
//     saveJsonToLocalStorage('secrets', secrets);
//     dispatch(setContext({ name: 'secrets', item: secrets }));
//   };
// }

const slice = createSlice({
  name: 'scripting',
  initialState,
  reducers: {
    setContext: (
      state,
      {
        payload,
      }: PayloadAction<{
        name: keyof ScriptContext;
        item: ScriptContext[keyof ScriptContext];
      }>
    ) => {
      state.context = { ...state.context, [payload.name]: payload.item };
    },
    setEntrypoint: (
      state,
      { payload }: PayloadAction<{ name: ScriptEntrypointNames; code: string }>
    ) => {
      const { name, code } = payload;
      if (code) {
        state.scripts.entrypoints[name].script = code;
      }
    },
    setSecret: (state, { payload }: PayloadAction<{ key: string; value: string }>) => {
      state.context.secrets[payload.key] = payload.value;
      saveJsonToLocalStorage('secrets', state.context.secrets);
    },
    removeSecret: (state, { payload }: PayloadAction<{ key: string }>) => {
      delete state.context.secrets[payload.key];
      saveJsonToLocalStorage('secrets', state.context.secrets);
    },
    setEntrypointEnabled: (
      state,
      { payload }: PayloadAction<{ name: ScriptEntrypointNames; enabled: boolean }>
    ) => {
      const { name, enabled } = payload;
      saveStringToLocalStorage(getEntrypointKeyName(name, 'enabled'), enabled ? 'true' : '');

      state.scripts.entrypoints[name].enabled = enabled;
    },
  },
});

export const selectRuneEntypoints = (store: RootState) => store.scripting.scripts.entrypoints;

export type ScriptingActionTypes = SliceActions<typeof slice.actions>;

export const { setEntrypoint, setEntrypointEnabled, setContext, setSecret, removeSecret } =
  slice.actions;

export default slice.reducer;
