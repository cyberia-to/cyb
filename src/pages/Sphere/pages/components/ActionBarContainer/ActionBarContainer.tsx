import { useState, useEffect, useMemo } from 'react';
import { coin } from '@cosmjs/launchpad';
import { useNavigate } from 'react-router-dom';
import BigNumber from 'bignumber.js';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';
import Button from 'src/components/btnGrd';
import { BASE_DENOM, MEMO_KEPLR } from 'src/constants/config';
import { useSphereContext } from 'src/pages/Sphere/Sphere.context';
import { Validator } from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';
import {
  Confirmed,
  TransactionSubmitted,
  TransactionError,
  Dots,
  ActionBar,
} from '../../../../../components';

import { LEDGER } from '../../../../../utils/config';
import Delegate from './components/Delegate/Delegate';
import ReDelegate from './components/ReDelegate/ReDelegate';

const {
  STAGE_INIT,
  STAGE_READY,
  STAGE_WAIT,
  STAGE_SUBMITTED,
  STAGE_CONFIRMING,
  STAGE_CONFIRMED,
  STAGE_ERROR,
} = LEDGER;

const fee = 'auto';
const TXTYPE_DELEGATE = 0;
const TXTYPE_UNDELEGATE = 1;
const TXTYPE_REDELEGATE = 2;
const LEDGER_GENERATION = 23;

function StatusTx({ stage, clearState, errorMessage, txHash, txHeight }) {
  if (stage === LEDGER_GENERATION) {
    return (
      <ActionBar>
        tx generation <Dots big />
      </ActionBar>
    );
  }

  if (stage === STAGE_WAIT) {
    return (
      <ActionBar>
        <div>
          approve tx
          <Dots big />
        </div>
      </ActionBar>
    );
  }

  if (stage === STAGE_SUBMITTED || stage === STAGE_CONFIRMING) {
    return <TransactionSubmitted />;
  }

  if (stage === STAGE_CONFIRMED) {
    return (
      <Confirmed
        txHash={txHash}
        txHeight={txHeight}
        onClickBtnClose={clearState}
      />
    );
  }

  if (stage === STAGE_ERROR && errorMessage !== null) {
    return (
      <TransactionError errorMessage={errorMessage} onClickBtn={clearState} />
    );
  }

  return null;
}

const getValidatorAddres = (validators) => {
  let validatorAddres = null;
  if (validators.operatorAddress) {
    validatorAddres = validators.operatorAddress;
  }
  if (validators.operator_address) {
    validatorAddres = validators.operator_address;
  }

  return validatorAddres;
};

const checkTxs = (response, updateState) => {
  console.log('response', response);
  const { setStage, setTxHash, setErrorMessage } = updateState;
  if (response.code === 0) {
    const hash = response.transactionHash;
    console.log('hash :>> ', hash);
    setStage(STAGE_SUBMITTED);
    setTxHash(hash);
  } else {
    setStage(STAGE_ERROR);
    setTxHash(null);
    setErrorMessage(response.rawLog.toString());
  }
};

const useCheckStatusTx = (txHash, setStage, setErrorMessage, updateFnc) => {
  const queryClient = useQueryClient();
  const [txHeight, setTxHeight] = useState(null);

  useEffect(() => {
    const confirmTx = async () => {
      if (queryClient && txHash !== null) {
        setStage(STAGE_CONFIRMING);
        const response = await queryClient.getTx(txHash);
        console.log('response :>> ', response);
        if (response && response !== null) {
          if (response.code === 0) {
            setStage(STAGE_CONFIRMED);
            setTxHeight(response.height);
            if (updateFnc) {
              updateFnc();
            }
            return;
          }
          if (response.code) {
            setStage(STAGE_ERROR);
            setTxHeight(response.height);
            setErrorMessage(response.rawLog);
            return;
          }
        }
        setTimeout(confirmTx, 1500);
      }
    };
    confirmTx();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryClient, txHash]);

  return { txHeight };
};

type Props = {
  validators?: Validator;
  updateFnc: () => void;
};

function ActionBarContainer({ validators, updateFnc }: Props) {
  const { signer, signingClient } = useSigningClient();
  const {
    validators: validatorsAll,
    balance,
    isFetchingBalance,
    delegationsData,
  } = useSphereContext();
  const queryClient = useQueryClient();
  const [stage, setStage] = useState(STAGE_INIT);
  const [txType, setTxType] = useState(null);
  const [errorMessage, setErrorMessage] = useState(null);
  const [txHash, setTxHash] = useState(null);
  const [amount, setAmount] = useState<string>('');
  const [valueSelect, setValueSelect] = useState('');
  const { txHeight } = useCheckStatusTx(
    txHash,
    setStage,
    setErrorMessage,
    updateFnc
  );

  const validatorSelected = validators?.operatorAddress;

  const staked =
    validatorSelected && delegationsData[validatorSelected]
      ? delegationsData[validatorSelected].amount
      : 0;

  const errorState = (error) => {
    setTxHash(null);
    setStage(STAGE_ERROR);
    setErrorMessage(error.toString());
  };

  useEffect(() => {
    clearFunc();
  }, [validatorSelected]);

  const clearFunc = () => {
    setTxHash(null);
    setAmount('');
    setValueSelect('');
    setErrorMessage(null);
    setTxType(null);
    setStage(STAGE_INIT);
  };

  const delegateTokens = async () => {
    if (signer && signingClient) {
      try {
        const [{ address: addressKeplr }] = await signer.getAccounts();
        const validatorAddres = getValidatorAddres(validators);

        setStage(STAGE_WAIT);
        const response = await signingClient.delegateTokens(
          addressKeplr,
          validatorAddres,
          coin(amount, BASE_DENOM),
          fee,
          MEMO_KEPLR
        );
        checkTxs(response, { setTxHash, setErrorMessage, setStage });
      } catch (error) {
        errorState(error);
      }
    }
  };

  const undelegateTokens = async () => {
    if (signer && signingClient) {
      try {
        const [{ address: addressKeplr }] = await signer.getAccounts();

        const validatorAddres = getValidatorAddres(validators);

        setStage(STAGE_WAIT);
        const response = await signingClient.undelegateTokens(
          addressKeplr,
          validatorAddres,
          coin(amount, BASE_DENOM),
          fee,
          MEMO_KEPLR
        );
        checkTxs(response, { setTxHash, setErrorMessage, setStage });
      } catch (error) {
        errorState(error);
      }
    }
  };

  const redelegateTokens = async () => {
    if (signer && signingClient) {
      try {
        const [{ address: addressKeplr }] = await signer.getAccounts();

        setStage(STAGE_WAIT);
        const validatorAddres = getValidatorAddres(validators);
        const response = await signingClient.redelegateTokens(
          addressKeplr,
          validatorAddres,
          valueSelect,
          coin(amount, BASE_DENOM),
          fee,
          MEMO_KEPLR
        );
        checkTxs(response, { setTxHash, setErrorMessage, setStage });
      } catch (error) {
        errorState(error);
      }
    }
  };

  const claimRewards = async () => {
    if (signer && signingClient && queryClient) {
      try {
        const [{ address: addressKeplr }] = await signer.getAccounts();
        const validatorAddress: string[] = [];

        setStage(LEDGER_GENERATION);
        const delegationTotalRewards = await queryClient.delegationTotalRewards(
          addressKeplr
        );
        if (delegationTotalRewards !== null && delegationTotalRewards.rewards) {
          const { rewards } = delegationTotalRewards;
          Object.keys(rewards).forEach((key) => {
            if (rewards[key].reward !== null) {
              validatorAddress.push(rewards[key].validatorAddress);
            }
          });

          setStage(STAGE_WAIT);
          const response = await signingClient.withdrawAllRewards(
            addressKeplr,
            validatorAddress,
            fee
          );
          checkTxs(response, { setTxHash, setErrorMessage, setStage });
        }
      } catch (error) {
        errorState(error);
      }
    }
  };

  const funcSetTxType = (type) => {
    setTxType(type);
    setStage(STAGE_READY);
  };

  const validRestakeBtn = parseFloat(amount) > 0 && valueSelect.length > 0;

  const validRewards = useMemo(() => {
    if (
      balance &&
      balance.frozen &&
      balance.frozen.amount !== 0 &&
      balance.growth &&
      balance.growth.amount !== 0
    ) {
      const delegation = new BigNumber(balance.frozen.amount);
      const rewards = new BigNumber(balance.growth.amount);
      const procentRewards = rewards
        .div(delegation)
        .multipliedBy(100)
        .toNumber();

      if (procentRewards > 0.01) {
        return true;
      }
      return false;
    }
    return false;
  }, [balance]);

  const amountChangeHandler = (values: string) => {
    setAmount(values);
  };

  const onClickBackToChoseHandler = () => {
    setStage(STAGE_INIT);
    setTxType(null);
    amountChangeHandler('');
  };

  // loadingBalanceInfo
  if (!validators && stage === STAGE_INIT && isFetchingBalance) {
    return <ActionBar text={<Dots />} />;
  }

  // stage balance.delegation === 0
  if (!validators && stage === STAGE_INIT && balance?.frozen.amount === 0) {
    return <ActionBar text="Choose hero to get H and earn rewards" />;
  }

  // stage balance.delegation === 0
  if (!validators && stage === STAGE_INIT) {
    return (
      <ActionBar
        button={{
          text: ' Claim rewards',
          disabled: !validRewards,
          onClick: claimRewards,
        }}
      />
    );
  }

  if (validators && stage === STAGE_INIT && txType === null) {
    return (
      <ActionBar
        button={{
          text: 'Stake',
          onClick: () => funcSetTxType(TXTYPE_DELEGATE),
        }}
      >
        <span>{validators.description.moniker}</span>
        {new BigNumber(staked).comparedTo(0) > 0 && (
          <div style={{ display: 'flex', alignItems: 'center' }}>
            <Button
              style={{
                margin: '0 25px',
              }}
              onClick={() => funcSetTxType(TXTYPE_UNDELEGATE)}
            >
              Unstake
            </Button>
            <Button onClick={() => funcSetTxType(TXTYPE_REDELEGATE)}>
              Restake
            </Button>
          </div>
        )}
      </ActionBar>
    );
  }

  if (
    stage === STAGE_READY &&
    (txType === TXTYPE_DELEGATE || txType === TXTYPE_UNDELEGATE)
  ) {
    return (
      <ActionBar
        onClickBack={onClickBackToChoseHandler}
        button={{
          text: 'Generate Tx',
          onClick:
            txType === TXTYPE_DELEGATE ? delegateTokens : undelegateTokens,
          disabled: !amount,
        }}
      >
        <Delegate
          moniker={validators ? validators.description.moniker : ''}
          onChangeInputAmount={amountChangeHandler}
          value={amount}
          maxValue={
            txType === TXTYPE_DELEGATE ? balance?.liquid.amount : staked
          }
          delegate={txType === TXTYPE_DELEGATE}
        />
      </ActionBar>
    );
  }

  if (stage === STAGE_READY && txType === TXTYPE_REDELEGATE) {
    return (
      <ActionBar
        onClickBack={onClickBackToChoseHandler}
        button={{
          text: 'Generate Tx',
          onClick: redelegateTokens,
          disabled: !validRestakeBtn,
        }}
      >
        <ReDelegate
          onChangeInputAmount={amountChangeHandler}
          amount={amount}
          validatorsAll={validatorsAll}
          validatorSelect={validators}
          maxValue={staked}
          onChangeReDelegate={(val) => setValueSelect(val)}
          valueSelect={valueSelect}
        />
      </ActionBar>
    );
  }

  if (stage !== STAGE_READY) {
    return (
      <StatusTx
        stage={stage}
        clearState={clearFunc}
        errorMessage={errorMessage}
        txHash={txHash}
        txHeight={txHeight}
      />
    );
  }

  return null;
}

export default ActionBarContainer;
