import type { Ctx } from '@milkdown/kit/ctx';
import { createSlice } from '@milkdown/kit/ctx';
import type { EditorFeature } from '../feature/feature';

export const FeaturesCtx = createSlice([] as EditorFeature[], 'FeaturesCtx');

export function configureFeatures(features: EditorFeature[]) {
  return (ctx: Ctx) => {
    ctx.inject(FeaturesCtx, features);
  };
}
