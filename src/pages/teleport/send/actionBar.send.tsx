import { Coin } from '@cosmjs/launchpad';
import BigNumber from 'bignumber.js';
import { useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useSigningClient } from 'src/contexts/signerClient';
import useSetActiveAddress from 'src/hooks/useSetActiveAddress';
import { useAppSelector } from 'src/redux/hooks';
import { RootState } from 'src/redux/store';
import { sendTokensWithMessage } from 'src/services/neuron/neuronApi';
import { Option } from 'src/types';
import { addIfpsMessageOrCid } from 'src/utils/ipfs/helpers';
import { Account, ActionBar as ActionBarCenter } from '../../../components';

import { LEDGER } from '../../../utils/config';
import { convertAmountReverce } from '../../../utils/utils';

import ActionBarPingTxs from '../components/actionBarPingTxs';

const { STAGE_INIT, STAGE_ERROR, STAGE_SUBMITTED } = LEDGER;

const coinFunc = (amount: number, denom: string): Coin => {
  return { denom, amount: new BigNumber(amount).toString(10) };
};

type Props = {
  tokenAmount: string;
  tokenSelect: string;
  recipient: string | undefined;
  updateFunc: () => void;
  isExceeded: boolean;
  memoValue: string;
};

function ActionBar({ stateActionBar }: { stateActionBar: Props }) {
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const { signingClient, signer } = useSigningClient();
  const { tracesDenom } = useIbcDenom();
  const [stage, setStage] = useState(STAGE_INIT);
  const [txHash, setTxHash] = useState<Option<string>>(undefined);
  const [errorMessage, setErrorMessage] = useState<Option<string | JSX.Element>>(undefined);
  const { senseApi, ipfsApi } = useBackend();

  const { tokenAmount, tokenSelect, recipient, updateFunc, isExceeded, memoValue } = stateActionBar;

  const sendOnClick = async () => {
    if (signer && signingClient && tracesDenom && recipient) {
      const [{ address }] = await signer.getAccounts();

      const [{ coinDecimals: coinDecimalsA }] = tracesDenom(tokenSelect);

      const amountTokenA = convertAmountReverce(tokenAmount, coinDecimalsA);

      setStage(STAGE_SUBMITTED);

      const offerCoin = [coinFunc(amountTokenA, tokenSelect)];

      if (addressActive !== null && addressActive.bech32 === address) {
        const memo = await addIfpsMessageOrCid(memoValue, { ipfsApi });
        await sendTokensWithMessage(address, recipient, offerCoin, memo, {
          senseApi,
          signingClient,
        })
          .then((txHash) => {
            setTxHash(txHash);
          })
          .catch((e) => {
            setTxHash(undefined);
            setStage(STAGE_ERROR);
            setErrorMessage(e.toString());
          });
      } else {
        setStage(STAGE_ERROR);
        setErrorMessage(
          <span>
            Add address <Account margin="0 5px" address={address} /> to your pocket or make active{' '}
          </span>
        );
      }
    }
  };

  const clearState = () => {
    setStage(STAGE_INIT);
    setTxHash(undefined);
    setErrorMessage(undefined);
  };

  if (stage === STAGE_INIT) {
    return (
      <ActionBarCenter
        button={{
          text: 'Send',
          onClick: sendOnClick,
          disabled: isExceeded,
        }}
      />
    );
  }

  const stageActionBarStaps = {
    stage,
    setStage,
    clearState,
    updateFunc,
    txHash,
    errorMessageProps: errorMessage,
  };

  return <ActionBarPingTxs stageActionBarStaps={stageActionBarStaps} />;
}

export default ActionBar;
