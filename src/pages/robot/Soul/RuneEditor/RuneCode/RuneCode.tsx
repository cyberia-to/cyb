import { Controlled as CodeMirror } from 'react-codemirror2';

import './formatting/rune';
import 'codemirror/lib/codemirror.css';
import 'codemirror/theme/tomorrow-night-eighties.css';

import './formatting/codeMirror.css';
import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from 'react';
import styles from './RuneCode.module.scss';

// const highlightErrors = (
//   codeMirror: CodeMirror,
//   diagnostics: any[],
//   styles: Record<string, string>
// ) => {
//   const cm = codeMirror!.editor;

//   cm.getAllMarks().forEach((mark) => mark.clear());

//   diagnostics.forEach((error) => {
//     const { start, end } = error;
//     cm.scrollIntoView({ line: start.line, ch: start.character });
//     cm.markText(
//       { line: start.line, ch: start.character },
//       { line: end.line, ch: end.character },
//       {
//         className: styles.errorHighlight,
//         clearOnEnter: true,
//       }
//     );
//   });
// };

type RuneCodeProps = {
  code: string;
  size: string;
  readOnly?: boolean;
  onChange?: (code: string) => void;
};

export type RuneCodeHandle = {
  highlightErrors: (diagnostics: any[]) => void;
};

const RuneCode = forwardRef<RuneCodeHandle, RuneCodeProps>(
  ({ code, readOnly = false, size = '400px', onChange }, ref) => {
    const codeMirrorRef = useRef<CodeMirror | null>(null);

    const [runeCode, setRuneCode] = useState<string>('');

    useEffect(() => {
      setRuneCode(code);
    }, [code]);

    const highlightErrors = useCallback((diagnostics: any[]) => {
      const cm = codeMirrorRef.current!.editor;

      cm.getAllMarks().forEach((mark) => mark.clear());

      diagnostics.forEach((error) => {
        const { start, end } = error;
        cm.scrollIntoView({ line: start.line, ch: start.character });
        cm.markText(
          { line: start.line, ch: start.character },
          { line: end.line, ch: end.character },
          {
            className: styles.errorHighlight,
            clearOnEnter: true,
          }
        );
      });
    }, []);

    useImperativeHandle(ref, () => ({
      highlightErrors,
    }));

    return (
      <CodeMirror
        ref={codeMirrorRef}
        editorDidMount={(editor) => {
          editor.setSize('', size);
        }}
        value={runeCode}
        options={{
          readOnly,
          mode: 'rune',
          theme: 'tomorrow-night-eighties',
          lineNumbers: true,
        }}
        onBeforeChange={(_editor, _data, value) => {
          setRuneCode(value);
          onChange?.(value);
        }}
      />
    );
  }
);

RuneCode.displayName = 'RuneCode';

export default RuneCode;
