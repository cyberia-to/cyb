import { ThunkAction } from 'redux-thunk';
import { RootState } from './store';
import { AnyAction } from 'redux';

export type AppThunk<ReturnType = void> = ThunkAction<ReturnType, RootState, unknown, AnyAction>;

export type SliceActions<T> = {
  [K in keyof T]: T[K] extends (...args: any[]) => infer A ? A : never;
}[keyof T];
