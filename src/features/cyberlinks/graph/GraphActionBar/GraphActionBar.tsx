import { useState } from 'react';
import { ActionBar as ActionBarComponent, Button, InputNumber } from 'src/components';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useGraphLimit from 'src/pages/robot/Brain/useGraphLimit';
import GraphFullscreenBtn, { useFullscreen } from '../../GraphFullscreenBtn/GraphFullscreenBtn';

enum Steps {
  INITIAL,
  CHANGE_LIMIT,
}

function GraphActionBar({ children }: { children?: React.ReactNode }) {
  const [step, setStep] = useState(Steps.INITIAL);
  const { isFullscreen } = useFullscreen();

  const { limit, setLimit } = useGraphLimit();
  const [newLimit, setNewLimit] = useState(limit);

  let content;
  let adviserText;
  let onClickBack;

  switch (step) {
    case Steps.INITIAL: {
      content = (
        <>
          {!isFullscreen && (
            <>
              {children}

              <Button onClick={() => setStep(Steps.CHANGE_LIMIT)}>Change limit</Button>
            </>
          )}

          <GraphFullscreenBtn />
        </>
      );

      break;
    }

    case Steps.CHANGE_LIMIT: {
      content = (
        <>
          <InputNumber
            value={newLimit}
            onChange={(value) => setNewLimit(Number(value))}
            placeholder="Enter new limit"
          />

          <Button
            onClick={() => {
              setLimit(newLimit);
              setStep(Steps.INITIAL);
            }}
          >
            Confirm
          </Button>
        </>
      );

      onClickBack = () => setStep(Steps.INITIAL);

      adviserText = 'change limit to show in graph, but it can reduce performance';

      break;
    }

    default: {
    }
  }

  useAdviserTexts({
    defaultText: adviserText,
  });

  return <ActionBarComponent onClickBack={onClickBack}>{content}</ActionBarComponent>;
}

export default GraphActionBar;
