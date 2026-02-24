import { findParent } from '@milkdown/kit/prose';
import type { Node } from '@milkdown/kit/prose/model';
import type { EditorState } from '@milkdown/kit/prose/state';
import { Plugin, PluginKey } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { $ctx, $prose } from '@milkdown/kit/utils';
import { isInCodeBlock, isInList } from '../../utils/utils';
import type { DefineFeature } from '../shared';

function isDocEmpty(doc: Node) {
  return doc.childCount <= 1 && !doc.firstChild?.content.size;
}

function createPlaceholderDecoration(
  state: EditorState,
  placeholderText: string
): Decoration | null {
  const { selection } = state;
  if (!selection.empty) {
    return null;
  }

  const $pos = selection.$anchor;
  const node = $pos.parent;
  if (node.content.size > 0) {
    return null;
  }

  const inTable = findParent((node) => node.type.name === 'table')($pos);
  if (inTable) {
    return null;
  }

  const before = $pos.before();

  return Decoration.node(before, before + node.nodeSize, {
    class: 'ProseMirror',
    'data-placeholder': placeholderText,
  });
}

interface PlaceholderConfig {
  text: string;
  mode: 'doc' | 'block';
}

export type PlaceHolderFeatureConfig = Partial<PlaceholderConfig>;

export const placeholderConfig = $ctx(
  {
    text: 'Start here ...',
    mode: 'doc',
  } as PlaceholderConfig,
  'placeholderConfigCtx'
);

export const placeholderPlugin = $prose((ctx) => {
  return new Plugin({
    key: new PluginKey('CREPE_PLACEHOLDER'),
    props: {
      decorations: (state) => {
        const config = ctx.get(placeholderConfig.key);
        if (config.mode === 'doc' && !isDocEmpty(state.doc)) {
          return null;
        }

        if (isInCodeBlock(state.selection) || isInList(state.selection)) {
          return null;
        }

        const placeholderText = config.text ?? 'Start here ...';
        const deco = createPlaceholderDecoration(state, placeholderText);
        if (!deco) {
          return null;
        }

        return DecorationSet.create(state.doc, [deco]);
      },
    },
  });
});

export const defineFeature: DefineFeature<PlaceHolderFeatureConfig> = (editor, config) => {
  editor
    .config((ctx) => {
      if (config) {
        ctx.update(placeholderConfig.key, (prev) => {
          return {
            ...prev,
            ...config,
          };
        });
      }
    })
    .use(placeholderPlugin)
    .use(placeholderConfig);
};
