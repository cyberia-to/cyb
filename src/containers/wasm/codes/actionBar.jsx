/* eslint-disable jsx-a11y/control-has-associated-label */

import { ActionBar as ActionBarContainer, Pane } from '@cybercongress/gravity';
import { useEffect, useRef, useState } from 'react';
import Button from 'src/components/btnGrd';
import AddFileButton from 'src/components/buttons/AddFile/AddFile';
import { MEMO_KEPLR } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';

import { getTxs } from 'src/services/transactions/lcd';
import {
  Account,
  ActionBarContentText,
  Confirmed,
  Dots,
  TransactionError,
  TransactionSubmitted,
} from '../../../components';
import { LEDGER } from '../../../utils/config';

const { STAGE_INIT, STAGE_ERROR, STAGE_CONFIRMING, STAGE_CONFIRMED, STAGE_SUBMITTED } = LEDGER;

function ActionBar({ updateFnc }) {
  const { signer, signingClient } = useSigningClient();
  const inputOpenFileRef = useRef();
  const [wasm, setWasm] = useState(null);
  const [stage, setStage] = useState(STAGE_INIT);
  const [txHash, setTxHash] = useState(null);
  const [errorMessage, setErrorMessage] = useState(null);
  const [txHeight, setTxHeight] = useState(null);

  const currentAddress = useCurrentAddress();

  // refactor
  const addressActive = {
    bech32: currentAddress,
  };

  useEffect(() => {
    const confirmTx = async () => {
      if (txHash !== null) {
        setStage(STAGE_CONFIRMING);
        const res = await getTxs(txHash);
        if (res) {
          const response = res.tx_response;
          if (response.logs) {
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
            setErrorMessage(response.raw_log);
            return;
          }
        }
        setTimeout(confirmTx, 1500);
      }
    };
    confirmTx();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [txHash, updateFnc]);

  const uploadCode = async () => {
    setStage(STAGE_SUBMITTED);
    if (signer && signingClient) {
      const wasmBytes = new Uint8Array(await wasm.arrayBuffer());
      try {
        const [{ address }] = await signer.getAccounts();

        if (addressActive !== null && addressActive.bech32 === address) {
          const response = await signingClient.upload(
            address,
            wasmBytes,
            Soft3MessageFactory.fee(2),
            MEMO_KEPLR
          );
          if (response.code === 0) {
            setTxHash(response.transactionHash);
          } else {
            setTxHash(null);
            setErrorMessage(response.rawLog.toString());
            setStage(STAGE_ERROR);
          }
        } else {
          setErrorMessage(
            <span>
              Add address <Account margin="0 5px" address={address} /> to your pocket or make active{' '}
            </span>
          );
          setStage(STAGE_ERROR);
        }
      } catch (e) {
        console.log(`e`, e);
        setStage(STAGE_ERROR);
        setErrorMessage(e.toString());
      }
    }
  };

  const onClickClear = () => {
    setWasm(null);
  };

  const showOpenFileDlg = () => {
    inputOpenFileRef.current.click();
  };

  const onFilePickerChange = (files) => {
    const file = files.current.files[0];
    setWasm(file);
  };

  const clearState = () => {
    setWasm(null);
    setStage(STAGE_INIT);
    setTxHash(null);
    setTxHeight(null);
    setErrorMessage(null);
  };

  if (stage === STAGE_INIT) {
    return (
      <ActionBarContainer>
        <Pane width="65%" display="flex">
          <ActionBarContentText>
            <div>{wasm?.name ? wasm.name : 'Select .wasm file'}</div>
            <input
              ref={inputOpenFileRef}
              onChange={() => onFilePickerChange(inputOpenFileRef)}
              type="file"
              accept=".wasm"
              style={{ display: 'none' }}
            />
            <AddFileButton isRemove={wasm} onClick={wasm ? onClickClear : showOpenFileDlg} />
          </ActionBarContentText>
          <Button disabled={wasm === null || !signer} onClick={() => uploadCode()}>
            Upload
          </Button>
        </Pane>
      </ActionBarContainer>
    );
  }

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

  if (stage === STAGE_ERROR && errorMessage !== null) {
    return <TransactionError errorMessage={errorMessage} onClickBtn={() => clearState()} />;
  }

  return null;
}

export default ActionBar;
