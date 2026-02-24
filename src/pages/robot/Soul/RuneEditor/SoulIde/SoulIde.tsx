import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { Option } from 'src/types';
import RuneCode, { RuneCodeHandle } from '../RuneCode/RuneCode';
import RuneOutput, { RuneOutputHandle } from '../RuneOutput/RuneOutput';
import styles from './SoulIde.module.scss';

type SoulIdeProps = {
  mainCode: string;
  readOnly: boolean;
  onChange?: (code: string) => void;
};

export type SoulIdeHandle = {
  test: (funcName: string, funcParams: any[]) => Promise<boolean>;
  save: () => Promise<Option<string>>;
  putToLog: (items: string[]) => void;
  clearLog: () => void;
};

const SoulIde = forwardRef<SoulIdeHandle, SoulIdeProps>(
  ({ mainCode, readOnly = false, onChange }, ref) => {
    const mainCodeRef = useRef<RuneCodeHandle | null>(null);
    const outputRef = useRef<RuneOutputHandle | null>(null);
    const { rune } = useBackend();

    const [code, setCode] = useState<string>('');
    const clearLog = () => outputRef.current?.clear();

    useEffect(() => {
      setCode(mainCode);
      outputRef.current?.clear();
      // setIsChanged(false);
    }, [mainCode]);

    const test = async (funcName: string, funcParams: any[] = []) => {
      outputRef.current?.scrollIntoView();

      outputRef.current?.put([`🚧 Execute your '${funcName}'.`]);
      return rune
        ?.run(code, {
          execute: true,
          funcName,
          funcParams,
        })
        .then((result) => {
          const isOk = !result.diagnosticsOutput && !result.error;
          mainCodeRef.current?.highlightErrors(result.diagnostics);
          if (!isOk) {
            outputRef.current?.put(['⚠️ Errors:', `   ${result.diagnosticsOutput}`]);
          } else {
            outputRef.current?.put(
              [
                '🏁 Result:',
                `   ${JSON.stringify(result.result)}`,
                '🧪 Raw output:',
                result?.output || 'no output.',
              ],
              '\r\n'
            );
          }

          return isOk;
        });
    };

    const save = async () => {
      clearLog();
      if (!code) {
        outputRef.current?.put([`🚫 Can't save empty code`]);
        return undefined;
      }

      return rune
        ?.run(code, {
          execute: false,
        })
        .then((result) => {
          if (result.diagnosticsOutput || result.error) {
            outputRef.current?.put(['⚠️ Errors:', `   ${result.diagnosticsOutput}`]);
            mainCodeRef.current?.highlightErrors(result.diagnostics);
            return undefined;
          }
          outputRef.current?.put(['🏁 Compiled - OK.']);
          return code;
        });
    };

    useImperativeHandle(ref, () => ({
      test,
      save,
      putToLog: (lines) => outputRef.current?.put(lines),
      clearLog,
    }));

    const setRuneCode = useCallback(
      (code: string) => {
        setCode(code);
        onChange?.(code);
      },
      [onChange]
    );

    return (
      <div className={styles.wrapper}>
        <RuneCode
          ref={mainCodeRef}
          code={code}
          readOnly={readOnly}
          size="400px"
          onChange={setRuneCode}
        />
        <div className={styles.separator} />
        <RuneOutput ref={outputRef} />
      </div>
    );
  }
);

SoulIde.displayName = 'SoulIde';
export default SoulIde;
