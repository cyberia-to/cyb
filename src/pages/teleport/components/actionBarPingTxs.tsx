import { ActionBar as ActionBarContainer } from '@cybercongress/gravity';
import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { Option } from 'src/types';
import {
  ActionBarContentText,
  Confirmed,
  Dots,
  TransactionError,
  TransactionSubmitted,
} from '../../../components';
import { LEDGER } from '../../../utils/config';

const { STAGE_ERROR, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED } = LEDGER;

function ActionBarPingTxs({ stageActionBarStaps }) {
  const queryClient = useQueryClient();
  const [txHeight, setTxHeight] = useState<Option<number>>(undefined);
  const [errorMessage, setErrorMessage] = useState<Option<string | JSX.Element>>(undefined);

  const { stage, setStage, clearState, txHash, errorMessageProps, updateFunc } =
    stageActionBarStaps;

  // Use ref for updateFunc to avoid restarting the polling loop when it changes
  const updateFuncRef = useRef(updateFunc);
  useEffect(() => {
    updateFuncRef.current = updateFunc;
  }, [updateFunc]);

  useEffect(() => {
    let retries = 0;
    let cancelled = false;
    const MAX_RETRIES = 20;

    const confirmTx = async () => {
      if (cancelled || !queryClient || !txHash) return;

      setStage(STAGE_CONFIRMING);
      try {
        const response = await queryClient.getTx(txHash);
        if (cancelled) return;
        if (response !== null) {
          if (response.code === 0) {
            setStage(STAGE_CONFIRMED);
            setTxHeight(response.height);
            if (updateFuncRef.current) {
              updateFuncRef.current();
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
      } catch (error) {
        console.error('getTx error:', error);
      }

      if (cancelled) return;
      retries += 1;
      if (retries >= MAX_RETRIES) {
        setStage(STAGE_ERROR);
        setErrorMessage(`transaction confirmation timed out after ${MAX_RETRIES * 1.5}s — check tx ${txHash} manually`);
        return;
      }
      setTimeout(confirmTx, 1500);
    };
    confirmTx();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryClient, txHash, setStage]);

  if (stage === STAGE_SUBMITTED) {
    return (
      <ActionBarContainer>
        <ActionBarContentText>
          check the transaction <Dots big />
        </ActionBarContentText>
      </ActionBarContainer>
    );
  }

  if (stage === STAGE_CONFIRMING) {
    return <TransactionSubmitted />;
  }

  if (stage === STAGE_CONFIRMED) {
    return <Confirmed txHash={txHash} txHeight={txHeight} onClickBtnClose={() => clearState()} />;
  }

  if ((stage === STAGE_ERROR && errorMessage !== null) || errorMessageProps) {
    return (
      <TransactionError
        errorMessage={errorMessage || errorMessageProps}
        onClickBtn={() => clearState()}
      />
    );
  }

  return null;
}

export default ActionBarPingTxs;
