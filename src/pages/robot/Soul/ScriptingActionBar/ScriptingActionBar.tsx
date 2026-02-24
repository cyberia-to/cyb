import { useState } from 'react';
// import { getIPFSContent } from 'src/utils/ipfs/utils-ipfs';
import { isCID } from 'src/utils/ipfs/helpers';

// import { Controlled as CodeMirror } from 'react-codemirror2';

import { Pane } from '@cybercongress/gravity';
import { Button, Input } from 'src/components';
import ActionBarContainer from 'src/components/actionBar';

// import { ScriptExecutionResult } from 'src/types/scripting';

import StepsBar from '../StepsBar/StepsBar';

// import { highlightErrors, compileScript } from '../utils';

import { useBackend } from 'src/contexts/backend/backend';
import styles from '../Soul.module.scss';

function ScriptingActionBar({
  isChanged,
  nickname,
  addToLog,
  onSaveClick,
  onCancelClick,
  resetPlayGround,
  resetToDefault,
  compileAndTest,
}: {
  isChanged: boolean;
  nickname: string;
  addToLog: (log: string[]) => void;
  onSaveClick: () => void;
  onCancelClick: () => void;
  resetPlayGround: () => void;
  resetToDefault: () => void;
  compileAndTest: (name: string, params?: any) => void;
}) {
  const [testCid, seTestCid] = useState('');
  const { ipfsApi } = useBackend();

  const [actionBarStep, setActionBarStep] = useState(0);

  const onTestMoonDomainClick = async () => {
    resetPlayGround();

    compileAndTest('moon_domain_resolver');
    setActionBarStep(0);
  };

  const onTestClick = async () => {
    resetPlayGround();

    if (!isCID(testCid)) {
      addToLog([`🚫 '${testCid}' - is not correct CID.`]);
      return;
    }
    addToLog(['💡 prepare data....', '', `🚧 fetching particle '${testCid}'...`]);

    const result = await ipfsApi?.fetchWithDetails(testCid);
    if (result) {
      const { type, content } = result;

      const preview = content!.length > 144 ? `${content!.slice(1, 144)}....` : content;

      addToLog([
        `   ☑️ content-type: ${type}`,
        `   ☑️ preview: ${preview}`,
        '',
        '💭 execute script....',
      ]);

      compileAndTest('personal_processor', [testCid, type, content]);
    } else {
      addToLog([`🚫 '${testCid}' - can't resolve.`]);
    }
    setActionBarStep(0);
  };

  const onTestPersonalProcessorClick = () => setActionBarStep(2);

  const onTestStepClick = () => setActionBarStep(1);

  const actionBarSteps = [
    <div key="step_0" className={styles.stepWraperJustified}>
      <Button onClick={onTestStepClick}>test cybscript</Button>
      <div>
        {isChanged && (
          <>
            <Button onClick={onSaveClick}>save</Button>
            <Button onClick={onSaveClick}>cancel</Button>
          </>
        )}
        {!isChanged && <Button onClick={resetToDefault}>reset to default</Button>}
      </div>
    </div>,
    <div key="step_1" className={styles.stepWrapper}>
      <Button onClick={onTestMoonDomainClick}>{`test ${nickname}.moon resolver`}</Button>
      <Button onClick={onTestPersonalProcessorClick}>test personal processor</Button>
    </div>,
    <div key="step_2" className={styles.stepWrapper}>
      <Input
        value={testCid}
        onChange={(e) => seTestCid(e.target.value)}
        placeholder="enter particle CID to apply script...."
      />
      <Button onClick={onTestClick}>Test particle processor</Button>
    </div>,
  ];

  return (
    <ActionBarContainer>
      <Pane className={styles.actionPanel}>
        <StepsBar
          steps={actionBarSteps}
          currentStep={actionBarStep}
          setCurrentStep={(value) => setActionBarStep(value)}
        />
      </Pane>
    </ActionBarContainer>
  );
}

export default ScriptingActionBar;
