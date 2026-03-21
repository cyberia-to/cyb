import { coins } from '@cosmjs/launchpad';
import { Pane } from '@cybercongress/gravity';
import BigNumber from 'bignumber.js';
import { ProposalStatus, VoteOption } from 'cosmjs-types/cosmos/gov/v1beta1/gov';
import { useEffect, useState } from 'react';
import { NumericFormat } from 'react-number-format';
import { BASE_DENOM, DEFAULT_GAS_LIMITS, MEMO } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { getTxs } from 'src/services/transactions/lcd';
import {
  Account,
  ActionBar,
  ActionBarContentText,
  BtnGrd,
  ButtonImgText,
  Confirmed,
  DenomArr,
  Dots,
  Input,
  Select,
  TransactionError,
  TransactionSubmitted,
} from '../../components';

// import styles from './ActionBarDetail.module.scss';

import { LEDGER } from '../../utils/config';
import { friendlyErrorMessage } from 'src/utils/errorMessages';

const imgCyber = require('../../image/blue-circle.png');

const { STAGE_INIT, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED, STAGE_ERROR } = LEDGER;

type Props = {
  proposals: any;
  id: string;
  update: () => void;
};

function ActionBarDetail({ proposals, id, update }: Props) {
  const { signer, signingClient } = useSigningClient();
  const [stage, setStage] = useState(STAGE_INIT);
  const [txHash, setTxHash] = useState(null);
  const [txHeight, setTxHeight] = useState(null);
  const [errorMessage, setErrorMessage] = useState(null);
  const [valueSelect, setValueSelect] = useState(1);
  const [valueDeposit, setValueDeposit] = useState('');

  const addressActive = useCurrentAddress();

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
            if (update) {
              update();
            }
            return;
          }
          if (response.code) {
            setStage(STAGE_ERROR);
            setTxHeight(response.height);
            setErrorMessage(friendlyErrorMessage(response.raw_log));
            return;
          }
        }
        setTimeout(confirmTx, 1500);
      }
    };
    confirmTx();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [txHash, update]);

  const clearState = () => {
    setStage(STAGE_INIT);
    setTxHash(null);
    setTxHeight(null);
    setErrorMessage(null);
    setValueDeposit('');
    setValueSelect(1);
  };

  const generateTx = async () => {
    if (signingClient && signer && Object.keys(proposals).length > 0) {
      try {
        const [{ address }] = await signer.getAccounts();
        if (addressActive === address) {
          let response = {};
          const fee = {
            amount: [],
            gas: DEFAULT_GAS_LIMITS.toString(),
          };

          setStage(STAGE_SUBMITTED);

          if (proposals.status === ProposalStatus.PROPOSAL_STATUS_VOTING_PERIOD) {
            response = await signingClient.voteProposal(address, id, valueSelect, fee, MEMO);
          }

          if (proposals.status === ProposalStatus.PROPOSAL_STATUS_DEPOSIT_PERIOD) {
            const amount = coins(parseFloat(valueDeposit), BASE_DENOM);
            response = await signingClient.depositProposal(address, id, amount, fee, MEMO);
          }

          if (response.code === 0) {
            setTxHash(response.transactionHash);
          } else {
            setTxHash(null);
            setErrorMessage(friendlyErrorMessage(response.rawLog));
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
        console.log(e);
        setTxHash(null);
        setErrorMessage(e.toString());
        setStage(STAGE_ERROR);
      }
    }
  };

  const onValueChangeDeposit = (values) => {
    setValueDeposit(new BigNumber(values).toNumber());
  };

  if (stage === STAGE_INIT && Object.keys(proposals).length === 0) {
    return (
      <ActionBar>
        <ActionBarContentText>
          <Dots />
        </ActionBarContentText>
      </ActionBar>
    );
  }

  if (stage === STAGE_INIT && proposals.status === ProposalStatus.PROPOSAL_STATUS_DEPOSIT_PERIOD) {
    return (
      <ActionBar>
        <ActionBarContentText>
          <Pane marginRight={10}>send Deposit</Pane>
          <DenomArr denomValue={BASE_DENOM} onlyImg />
          <div style={{ margin: '0 10px' }}>
            <NumericFormat
              value={valueDeposit}
              onValueChange={(values) => onValueChangeDeposit(values.value)}
              customInput={Input}
              thousandsGroupStyle="thousand"
              thousandSeparator=" "
              decimalScale={3}
              autoComplete="off"
              allowLeadingZeros
            />
          </div>
        </ActionBarContentText>
        <BtnGrd
          text="Deposit"
          disabled={!parseFloat(valueDeposit) > 0}
          onClick={() => generateTx()}
        />
      </ActionBar>
    );
  }

  if (stage === STAGE_INIT && proposals.status === ProposalStatus.PROPOSAL_STATUS_VOTING_PERIOD) {
    return (
      <ActionBar>
        <ActionBarContentText>
          <Select
            color="green"
            width={200}
            options={[
              { value: String(VoteOption.VOTE_OPTION_YES), text: 'Yes' },
              { value: String(VoteOption.VOTE_OPTION_NO), text: 'No' },
              {
                value: String(VoteOption.VOTE_OPTION_ABSTAIN),
                text: 'Abstain',
              },
              {
                value: String(VoteOption.VOTE_OPTION_NO_WITH_VETO),
                text: 'No With Veto',
              },
            ]}
            valueSelect={String(valueSelect)}
            onChangeSelect={(value) => setValueSelect(Number(value))}
          />
        </ActionBarContentText>
        <BtnGrd
          onClick={() => generateTx()}
          text="Vote"
        />
      </ActionBar>
    );
  }

  if (stage === STAGE_SUBMITTED) {
    return (
      <ActionBar>
        <ActionBarContentText>
          check the transaction <Dots big />
        </ActionBarContentText>
      </ActionBar>
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

export default ActionBarDetail;
