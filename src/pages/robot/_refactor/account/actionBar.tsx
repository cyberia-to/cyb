/* eslint-disable */
import { ActionBar } from '@cybercongress/gravity';
import React, { Component } from 'react';
import { createSearchParams } from 'react-router-dom';
import { CID_FOLLOW, CID_TWEET } from 'src/constants/app';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { useBackend } from 'src/contexts/backend/backend';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';
import { routes } from 'src/routes';
import { sendCyberlink } from 'src/services/neuron/neuronApi';
import { getTxs } from 'src/services/transactions/lcd';
import { AccountValue } from 'src/types/defaultAccount';
import {
  ActionBar as ActionBarComp,
  ActionBarContentText,
  Button,
  Confirmed,
  Dots,
  RewardsDelegators,
  StartStageSearchActionBar,
  TransactionError,
  TransactionSubmitted,
} from '../../../../components';
import withIpfsAndKeplr from '../../../../hocs/withIpfsAndKeplr';
import { LEDGER } from '../../../../utils/config';

const { STAGE_INIT, STAGE_READY, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED, STAGE_ERROR } =
  LEDGER;

type Props = {
  defaultAccount: AccountValue;

  type: string;

  addressSend: string;

  // can be followed
  follow: boolean;
  tweets: boolean;

  signer: ReturnType<typeof useSigningClient>['signer'];
  signingClient: ReturnType<typeof useSigningClient>['signingClient'];
  ipfsApi: ReturnType<typeof useBackend>['ipfsApi'];
  senseApi: ReturnType<typeof useBackend>['senseApi'];
  queryClient: ReturnType<typeof useQueryClient>;

  updateAddress: () => void;

  // add more
};

class ActionBarContainer extends Component<Props> {
  constructor(props: Props) {
    super(props);
    this.state = {
      stage: STAGE_INIT,
      address: null,
      file: null,
      toSend: '',
      toSendAddres: '',
      contentHash: '',
      txMsg: null,
      txHash: null,
      txHeight: null,
      rewards: null,
      errorMessage: null,
      bandwidth: {
        remained: 0,
        max_value: 0,
      },
    };
    this.timeOut = null;
    this.inputOpenFileRef = React.createRef();
  }

  calculationIpfsTo = async (contentHash) => {
    const { file } = this.state;
    const { ipfsApi } = this.props;

    let toCid = contentHash;

    if (file !== null) {
      toCid = await ipfsApi.addContent(file);
    } else if (!contentHash.match(PATTERN_IPFS_HASH)) {
      toCid = await ipfsApi.addContent(contentHash);
    }

    return toCid;
  };

  generateTxSendKplr = async () => {
    const { contentHash, toSendAddres, toSend } = this.state;
    const {
      type,
      addressSend,
      follow,
      tweets,
      signer,
      signingClient,
      ipfsApi,
      senseApi,
      queryClient,
    } = this.props;
    // const amount = parseFloat(toSend) * DIVISOR_CYBER_G;
    // const fee = {
    //   amount: [],
    //   gas: DEFAULT_GAS_LIMITS.toString(),
    // };

    if (signer && signingClient) {
      const [{ address }] = await signer.getAccounts();
      try {
        let txHash = null;

        if (type === 'security') {
          if (address === addressSend) {
            if (!queryClient) {
              return;
            }

            const dataTotalRewards = await queryClient.delegationTotalRewards(address);

            if (dataTotalRewards?.rewards) {
              const { rewards } = dataTotalRewards;
              const validatorAddress = [];
              Object.keys(rewards).forEach((key) => {
                if (rewards[key].reward) {
                  validatorAddress.push(rewards[key].validatorAddress);
                }
              });

              const gasLimitsRewards = 100000 * Object.keys(validatorAddress).length;
              const feeRewards = {
                amount: [],
                gas: gasLimitsRewards.toString(),
              };

              const response = await signingClient.withdrawAllRewards(
                address,
                validatorAddress,
                feeRewards
              );

              txHash = response.transactionHash;

              if (response.code) {
                throw Error(response.rawLog.toString());
              }
            }
          }
        } else if (type === 'log' || !type) {
          if (follow) {
            const toCid = await ipfsApi.addContent(addressSend);
            txHash = await sendCyberlink(address, CID_FOLLOW, toCid, {
              signingClient,
              senseApi,
            });
          } else if (tweets) {
            const toCid = await this.calculationIpfsTo(contentHash);
            txHash = await sendCyberlink(address, CID_TWEET, toCid, {
              signingClient,
              senseApi,
            });
          }
        }

        // sholdn't be after refactoring
        if (!txHash) {
          throw new Error('Tx hash is empty');
        }

        this.setState({ stage: STAGE_SUBMITTED, txHash });

        this.timeOut = setTimeout(this.confirmTx, 1500);
      } catch (e) {
        this.setState({
          txHash: null,
          stage: STAGE_ERROR,
          errorMessage: e.message,
        });
      }
    }
  };

  confirmTx = async () => {
    const { updateAddress } = this.props;
    const { txHash } = this.state;
    if (txHash !== null) {
      this.setState({ stage: STAGE_CONFIRMING });
      const res = await getTxs(txHash);
      if (res) {
        const data = res.tx_response;
        if (data.logs) {
          this.setState({
            stage: STAGE_CONFIRMED,
            txHeight: data.height,
          });
          if (updateAddress) {
            updateAddress();
          }
          return;
        }
        if (data.code) {
          this.setState({
            stage: STAGE_ERROR,
            txHeight: data.height,
            errorMessage: data.raw_log,
          });
          return;
        }
      }
    }
    this.timeOut = setTimeout(this.confirmTx, 1500);
  };

  onChangeInputAmount = (e) => {
    this.setState({
      toSend: e.target.value,
    });
  };

  onChangeInputInputAddressT = (e) => {
    this.setState({
      toSendAddres: e.target.value,
    });
  };

  onClickSend = () => {
    const { defaultAccount } = this.props;
    if (defaultAccount.keys === 'keplr') {
      this.generateTxSendKplr();
    }
  };

  clearState = () => {
    this.setState({
      stage: STAGE_INIT,
      address: null,
      toSend: '',
      toSendAddres: '',
      txMsg: null,
      txHash: null,
      txHeight: null,
      contentHash: '',
    });
    this.timeOut = null;
  };

  onChangeInput = async (e) => {
    const { value } = e.target;
    this.setState({
      contentHash: value,
    });
  };

  showOpenFileDlg = () => {
    this.inputOpenFileRef.current.click();
  };

  onFilePickerChange = (files) => {
    const file = files.current.files[0];

    this.setState({
      file,
    });
  };

  onClickClear = () => {
    this.setState({
      file: null,
    });
  };

  render() {
    const { type, addressSend, tweets, follow, defaultAccount } = this.props;
    const {
      stage,
      address,
      txMsg,
      txHash,
      txHeight,
      rewards,
      bandwidth,
      contentHash,
      errorMessage,
      file,
    } = this.state;

    const isOwner = defaultAccount && defaultAccount.bech32 === addressSend;

    if (stage === STAGE_INIT) {
      const followBtn = (
        <Button key={'action-bar-button-follow'} onClick={this.onClickSend}>
          Follow
        </Button>
      );

      const content = [];

      // main page
      if (!type) {
        if (!isOwner) {
          content.push(
            <Button
              link={`${routes.teleport.send.path}?${createSearchParams({
                recipient: addressSend,
                token: 'boot',
                amount: '1',
              }).toString()}`}
            >
              Send
            </Button>
          );
        }

        // if (follow) {
        //   content.push(followBtn);
        // }
      }

      if (!type) {
        if (follow) {
          content.push(followBtn);
        }

        if (tweets) {
          return (
            <StartStageSearchActionBar
              onClickBtn={this.onClickSend}
              contentHash={file !== null && file !== undefined ? file.name : contentHash}
              onChangeInputContentHash={this.onChangeInput}
              textBtn="Tweet"
              placeholder="What's happening?"
              inputOpenFileRef={this.inputOpenFileRef}
              showOpenFileDlg={this.showOpenFileDlg}
              onChangeInput={this.onFilePickerChange}
              onClickClear={this.onClickClear}
              file={file}
              keys={defaultAccount !== null ? defaultAccount.keys : false}
            />
          );
        }
      }

      if (type === 'security' && isOwner && defaultAccount.keys === 'keplr') {
        content.push(
          <Button
            key={'action-bar-button-claim-rewards'}
            onClick={this.onClickSend}
          >
            Claim rewards
          </Button>
        );
      }

      return <ActionBarComp>{content}</ActionBarComp>;
    }

    if (stage === STAGE_READY) {
      if (type === 'security') {
        return (
          <RewardsDelegators
            data={rewards}
            onClickBtnClose={this.clearState}
            onClickBtn={this.generateTx}
          />
        );
      }

      if (type === 'log') {
        return (
          <ActionBar>
            <ActionBarContentText>
              transaction generation <Dots big />
            </ActionBarContentText>
          </ActionBar>
        );
      }
    }

    if (stage === STAGE_SUBMITTED || stage === STAGE_CONFIRMING) {
      return <TransactionSubmitted onClickBtnClose={this.clearState} />;
    }

    if (stage === STAGE_CONFIRMED) {
      return (
        <Confirmed
          txHash={txHash}
          txHeight={txHeight}
          onClickBtn={this.clearState}
          onClickBtnClose={this.clearState}
        />
      );
    }

    if (stage === STAGE_ERROR && errorMessage !== null) {
      return (
        <TransactionError
          errorMessage={errorMessage}
          onClickBtn={this.clearState}
          onClickBtnClose={this.clearState}
        />
      );
    }

    return null;
  }
}

export default withIpfsAndKeplr(ActionBarContainer);
