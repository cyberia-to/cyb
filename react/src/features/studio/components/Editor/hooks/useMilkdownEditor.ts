import { defaultValueCtx, Editor, editorViewOptionsCtx, rootCtx } from '@milkdown/kit/core';
import { clipboard } from '@milkdown/kit/plugin/clipboard';
import { history, historyKeymap } from '@milkdown/kit/plugin/history';
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
import { commonmark } from '@milkdown/kit/preset/commonmark';
import { automd } from '@milkdown/plugin-automd';
import { useEditor } from '@milkdown/react';
import { nord } from '@milkdown/theme-nord';
import useDebounce from 'src/hooks/useDebounce';
import { placeholderConfig, placeholderPlugin } from '../feature/placeholder/placeholder';

// import {
//   inputRuleAsk,
//   inputRuleNeuron,
//   markSchemaCybSyntax,
//   remarkCybSyntaxPlugin,
// } from '../plugins/cybSyntax/pluginCybSyntax';

function useMilkdownEditor(defaultValue: string, onChange: (markdown: string) => void) {
  const { debounce } = useDebounce();

  const editorInfo = useEditor(
    (root) => {
      return Editor.make()
        .config((ctx) => {
          ctx.set(rootCtx, root);
          ctx.set(defaultValueCtx, defaultValue);
          ctx.set(editorViewOptionsCtx, { editable: () => true });
          ctx.get(listenerCtx).markdownUpdated((_, markdown) => {
            // console.log('markdown', markdown);
            debounce(onChange, 100)(markdown);
          });
          ctx.set(historyKeymap.key, {
            // Remap to one shortcut.
            Undo: 'Mod-z',
            // Remap to multiple shortcuts.
            Redo: ['Mod-y', 'Shift-Mod-z'],
          });
          ctx.get(placeholderConfig.key);
        })
        .config(nord)
        .use(commonmark)
        .use(clipboard)
        .use(automd)
        .use(history)
        .use(listener)
        .use(placeholderPlugin)
        .use(placeholderConfig);
    },
    [onChange, defaultValue]
  );

  return editorInfo;
}

export default useMilkdownEditor;
