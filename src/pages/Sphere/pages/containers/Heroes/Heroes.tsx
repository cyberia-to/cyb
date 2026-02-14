import { useMemo, useState } from 'react';

import { DenomArr } from 'src/components';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';

import BigNumber from 'bignumber.js';
import { useSphereContext } from 'src/pages/Sphere/Sphere.context';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { InfoBalance, ValidatorTable } from './components';
import styles from './Heroes.module.scss';
import reduceValidatorData from './utils/reduceValidatorData';
import { ValidatorTableData } from '../../../types/tableData';
import ActionBarContainer from '../../components/ActionBarContainer/ActionBarContainer';

function Heroes() {
  // const { status = 'active' } = useParams();
  const {
    bondedTokens,
    delegationsData,
    unbondingDays,
    stakingProvisions,
    balance,
    isFetchingBalance,
    validators,
    refetchFunc,
  } = useSphereContext();

  const [validatorSelect, setValidatorSelect] = useState<ValidatorTableData>();

  useAdviserTexts({
    defaultText: (
      <div className={styles.info}>
        {unbondingDays && (
          <>
            the current undelegation period is{' '}
            <strong>{unbondingDays} days</strong>
            <br />
          </>
        )}
        you need to burn 1 <DenomArr denomValue={DENOM_LIQUID} onlyImg /> to
        unstake 1 <DenomArr denomValue={BASE_DENOM} onlyImg />
      </div>
    ),
  });

  const estimatedApr = useMemo(() => {
    if (!stakingProvisions || !bondedTokens) {
      return undefined;
    }
    return new BigNumber(stakingProvisions)
      .dividedBy(bondedTokens)
      .multipliedBy(100)
      .dp(2, BigNumber.ROUND_FLOOR)
      .toString();
  }, [stakingProvisions, bondedTokens]);

  const updateFnc = () => {
    setValidatorSelect(undefined);
    refetchFunc();
  };

  return (
    <>
      <InfoBalance
        balance={balance}
        apr={estimatedApr}
        isFetchingBalance={isFetchingBalance}
      />
      <ValidatorTable
        data={reduceValidatorData(validators, {
          bondedTokens,
          delegationsData,
          stakingProvisions,
        })}
        onSelectRow={setValidatorSelect}
      />

      <ActionBarContainer updateFnc={updateFnc} validators={validatorSelect} />
    </>
  );
}

export default Heroes;
