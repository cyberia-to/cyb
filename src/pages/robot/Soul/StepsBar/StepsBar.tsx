import React from 'react';

import { ButtonIcon } from 'src/components';
import back from 'src/image/arrow-left-img.svg';

import styles from './StepsBar.module.scss';

function StepsBar({
  steps,
  currentStep,
  setCurrentStep,
}: {
  steps: React.ReactNode[];
  currentStep: number;
  setCurrentStep: (step: number) => void;
}) {
  const onBack = () => setCurrentStep(currentStep - 1);

  return (
    <div className={styles.stepsPanel}>
      {!!currentStep && <ButtonIcon img={back} onClick={onBack} text="previous step" />}
      {steps[currentStep]}
    </div>
  );
}

export default StepsBar;
