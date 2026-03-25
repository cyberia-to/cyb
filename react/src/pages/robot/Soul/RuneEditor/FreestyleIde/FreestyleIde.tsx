import { proxy } from 'comlink';
import { useRef, useState } from 'react';
import {
  ActionBar,
  Button,
  ContainerGradientText,
  DisplayTitle,
  MainContainer,
} from 'src/components';
import { useBackend } from 'src/contexts/backend/backend';
import defaultAiScript from 'src/services/scripting/rune/default/ai.rn';
import defaultPlaygroundScript from 'src/services/scripting/rune/default/playground.rn';
import RuneCode, { RuneCodeHandle } from '../RuneCode/RuneCode';
import RuneOutput, { RuneOutputHandle } from '../RuneOutput/RuneOutput';
import styles from './Freestyle.module.scss';

function FreestyleIde() {
  const libCodeRef = useRef<RuneCodeHandle | null>(null);
  const outputRef = useRef<RuneOutputHandle | null>(null);
  const testCodeRef = useRef<RuneCodeHandle | null>(null);

  const { rune } = useBackend();

  const [libCode, setLibCode] = useState<string>(defaultAiScript);
  const [testCode, setTestCode] = useState<string>(defaultPlaygroundScript);

  const runCode = async () => {
    const outputLog = outputRef.current;
    outputLog?.clear();
    outputLog?.scrollIntoView();

    outputLog?.put(['🚧 executing...']);
    return rune
      ?.run(
        testCode,
        {
          execute: true,
          funcName: 'main',
          funcParams: [],
        },
        proxy((result) => outputLog?.put([`callback: ${result}`])),

        { customRuntime: libCode }
      )
      .then((result) => {
        const isOk = !result.diagnosticsOutput && !result.error;
        if (result.diagnostics.length > 0) {
          if (result.diagnostics.at(0)?.name === 'main_entry') {
            testCodeRef.current?.highlightErrors(result.diagnostics);
          } else {
            libCodeRef.current?.highlightErrors(result.diagnostics);
          }
        }

        if (!isOk) {
          outputLog?.put(['⚠️ errors:', `   ${result.diagnosticsOutput || result.error}`]);
        } else {
          outputLog?.put(
            [
              '🏁 result:',
              `   ${JSON.stringify(result.result)}`,
              '🧪 raw output:',
              result?.output || 'no output.',
            ],
            '\r\n'
          );
        }

        return isOk;
      });
  };

  const _save = async () => {
    outputRef.current?.put([`saving not implemented yet.`]);
  };

  return (
    <>
      <MainContainer>
        <ContainerGradientText title={<DisplayTitle title="IDE" />}>
          <div className={styles.header}>library code</div>
          <RuneCode
            ref={libCodeRef}
            code={libCode}
            size="400px"
            onChange={(code) => setLibCode(code)}
          />
          <div className={styles.header}>playground</div>
          <RuneCode
            ref={testCodeRef}
            code={testCode}
            size="200px"
            onChange={(code) => setTestCode(code)}
          />
          <div className={styles.header}>output</div>
          <RuneOutput ref={outputRef} />
        </ContainerGradientText>
      </MainContainer>
      <ActionBar>
        <Button onClick={runCode}>Run</Button>
        <Button>Save</Button>
      </ActionBar>
    </>
  );
}

export default FreestyleIde;
