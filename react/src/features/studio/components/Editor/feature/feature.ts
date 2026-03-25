import type { Editor } from '@milkdown/kit/core';
import type { PlaceHolderFeatureConfig } from './placeholder/placeholder';

export enum EditorFeature {
  Placeholder = 'placeholder',
}

export interface CrepeFeatureConfig {
  [EditorFeature.Placeholder]?: PlaceHolderFeatureConfig;
}

export const defaultFeatures: Record<EditorFeature, boolean> = {
  [EditorFeature.Placeholder]: true,
};

async function loadFeature(feature: EditorFeature, editor: Editor, config?: never) {
  switch (feature) {
    case EditorFeature.Placeholder: {
      const { defineFeature } = await import('./placeholder/placeholder');
      return defineFeature(editor, config);
    }
  }
}

export default loadFeature;
