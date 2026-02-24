import { coin } from '@cosmjs/launchpad';
import { useState } from 'react';
import { DENOM_LIQUID } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import useWaitForTransaction, { Props as PropsTx } from 'src/hooks/useWaitForTransaction';
import { investmint } from 'src/services/neuron/neuronApi';
import {
  ActionBar as ActionBarContainer,
  Confirmed,
  Dots,
  TransactionError,
  TransactionSubmitted,
} from '../../components';
import { LEDGER } from '../../utils/config';
import { SelectedState } from './types';

const { STAGE_INIT, STAGE_ERROR, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED } = LEDGER;

const BASE_VESTING_TIME = 86401;

type Props = {
  amountH: number;
  resource: SelectedState;
  valueDays: number;
  resourceAmount: number;
  updateFnc?: () => void;
};

function ActionBar({ amountH, resource, valueDays, resourceAmount, updateFnc }: Props) {
  const { signer, signingClient } = useSigningClient();
  const [stage, setStage] = useState(STAGE_INIT);
  const [tx, setTx] = useState<PropsTx>();
  const [errorMessage, setErrorMessage] = useState(null);

  useWaitForTransaction({ hash: tx?.hash, onSuccess: tx?.onSuccess });

  const investmintFunc = async () => {
    if (!signer || !signingClient) {
      return;
    }

    setStage(STAGE_SUBMITTED);
    const [{ address }] = await signer.getAccounts();

    await investmint(
      address,
      coin(amountH, DENOM_LIQUID),
      resource,
      BASE_VESTING_TIME * valueDays,
      signingClient
    )
      .then((txHash) => {
        setStage(STAGE_CONFIRMING);

        setTx({
          hash: txHash,
          onSuccess: () => {
            updateFnc?.();
            setStage(STAGE_CONFIRMED);
          },
        });
      })
      .catch((e) => {
        console.log(e);

        setTx(undefined);
        setErrorMessage(e.toString());
        setStage(STAGE_ERROR);
      });
  };

  const clearState = () => {
    setStage(STAGE_INIT);
    setTx(undefined);
    setErrorMessage(null);
  };

  if (stage === STAGE_INIT) {
    return (
      <ActionBarContainer
        button={{
          text: 'Investmint',
          onClick: investmintFunc,
          disabled: resourceAmount === 0,
        }}
      />
    );
  }

  if (stage === STAGE_SUBMITTED) {
    return (
      <ActionBarContainer
        text={
          <>
            check the transaction <Dots big />
          </>
        }
      />
    );
  }

  if (stage === STAGE_CONFIRMING) {
    return <TransactionSubmitted />;
  }

  if (stage === STAGE_CONFIRMED) {
    return <Confirmed txHash={tx?.hash} onClickBtnClose={() => clearState()} />;
  }

  if (stage === STAGE_ERROR && errorMessage !== null) {
    return <TransactionError errorMessage={errorMessage} onClickBtn={() => clearState()} />;
  }

  return <ActionBarContainer />;
}

export default ActionBar;
