/* eslint-disable */

import { ActionBar, Pane } from '@cybercongress/gravity';
import React, { Component } from 'react';
import { connect } from 'react-redux';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { SenseApi } from 'src/contexts/backend/services/senseApi';
import withIpfsAndKeplr from 'src/hocs/withIpfsAndKeplr';
import { BackgroundWorker } from 'src/services/backend/workers/background/worker';
import { sendCyberlink } from 'src/services/neuron/neuronApi';
import { getTxs } from 'src/services/transactions/lcd';
import { DefaultAccount } from 'src/types/defaultAccount';
import {
  ActionBarContentText,
  ButtonImgText,
  Confirmed,
  Dots,
  StartStageSearchActionBar,
  TransactionError,
  TransactionSubmitted,
} from '../../components';
import { LEDGER } from '../../utils/config';
import { trimString } from '../../utils/utils';

const imgKeplr = require('../../image/keplr-icon.svg');
const imgLedger = require('../../image/ledger.svg');
const imgWallet = require('../../image/wallet-outline.svg');
const imgCyber = require('../../image/blue-circle.png');

const { STAGE_INIT, STAGE_READY, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED, STAGE_ERROR } =
  LEDGER;

const STAGE_IPFS_HASH = 3.1;
const STAGE_KEPLR_APPROVE = 3.2;

// generated
interface Props {
  defaultAccount: DefaultAccount;

  textBtn?: string;
  placeholder?: string;
  rankLink?: string;
  update: () => void;
  signer: any;
  ipfsApi: BackgroundWorker['ipfsApi'];
  senseApi: SenseApi;
  signingClient: any;
  keywordHash: string;
}

// TODO: REFACT
class ActionBarContainer extends Component<Props, any> {
  constructor(props: Props) {
    super(props);
    this.state = {
      stage: STAGE_INIT,
      addressLocalStor: null,
      contentHash: '',
      txHeight: null,
      txHash: null,
      errorMessage: null,
      file: null,
      fromCid: null,
      toCid: null,
    };
    this.timeOut = null;
    this.inputOpenFileRef = React.createRef();
    this.transport = null;
  }

  componentDidMount() {
    this.checkAddressLocalStorage();
  }

  componentDidUpdate(prevProps) {
    const { stage, fromCid, toCid } = this.state;
    const { defaultAccount } = this.props;

    if (stage === STAGE_IPFS_HASH) {
      if (toCid !== null && fromCid !== null) {
        this.generateTx();
      }
    }
    if (prevProps.defaultAccount.name !== defaultAccount.name) {
      this.checkAddressLocalStorage();
    }
  }

  checkAddressLocalStorage = async () => {
    const { defaultAccount } = this.props;
    const { account } = defaultAccount;
    if (account !== null && Object.hasOwn(account, 'cyber')) {
      const { keys, bech32 } = account.cyber;
      if (keys !== 'read-only') {
        this.setState({
          addressLocalStor: { address: bech32, keys },
        });
      } else {
        this.setState({
          addressLocalStor: null,
        });
      }
    } else {
      this.setState({
        addressLocalStor: null,
      });
    }
  };

  calculationIpfsTo = async () => {
    const { contentHash, file } = this.state;
    const { ipfsApi } = this.props;
    let content = '';
    let toCid;

    content = contentHash;
    if (file !== null) {
      content = file;
    }
    console.log('toCid', content);
    if (file === null && content.match(PATTERN_IPFS_HASH)) {
      toCid = content;
    } else {
      toCid = await ipfsApi.addContent(content);
    }

    this.setState({
      toCid,
    });
  };

  calculationIpfsFrom = async () => {
    const { keywordHash, ipfsApi } = this.props;

    let fromCid = keywordHash;

    if (!fromCid.match(PATTERN_IPFS_HASH)) {
      fromCid = await ipfsApi.addContent(fromCid);
    }

    this.setState({
      fromCid,
    });
  };

  onClickInitKeplr = () => {
    this.calculationIpfsFrom();
    this.calculationIpfsTo();
    this.setState({
      stage: STAGE_IPFS_HASH,
    });
  };

  generateTx = async () => {
    try {
      const { signer, signingClient, senseApi } = this.props;
      const { fromCid, toCid, addressLocalStor } = this.state;

      this.setState({
        stage: STAGE_KEPLR_APPROVE,
      });
      if (signer && signingClient) {
        const { address } = (await signer.getAccounts())[0];

        console.log('address', address);
        if (addressLocalStor !== null && addressLocalStor.address === address) {
          const _txHash = await sendCyberlink(address, fromCid, toCid, {
            signingClient,
            senseApi,
          })
            .then((txHash) => {
              console.log('hash :>> ', txHash);
              this.setState({ stage: STAGE_SUBMITTED, txHash });
              this.timeOut = setTimeout(this.confirmTx, 1500);
            })
            .catch((e) => {
              this.setState({
                txHash: null,
                stage: STAGE_ERROR,
                errorMessage: e.message,
              });
              console.log('result: ', e.message, e);
            });
        } else {
          this.setState({
            stage: STAGE_ERROR,
            errorMessage: `Add address ${trimString(address, 9, 5)} to your pocket or make active `,
          });
        }
      }
    } catch (e) {
      console.log(`e`, e);
      this.setState({
        stage: STAGE_ERROR,
        txBody: null,
        errorMessage: e.toString(),
      });
    }
  };

  confirmTx = async () => {
    const { update } = this.props;
    if (this.state.txHash !== null) {
      this.setState({ stage: STAGE_CONFIRMING });
      const res = await getTxs(this.state.txHash);

      if (res) {
        const data = res.tx_response;
        if (data.logs) {
          this.setState({
            stage: STAGE_CONFIRMED,
            txHeight: data.height,
          });
          if (update) {
            update();
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

  onChangeInput = async (e) => {
    const { value } = e.target;
    this.setState({
      contentHash: value,
    });
  };

  clearState = () => {
    this.setState({
      stage: STAGE_INIT,
      contentHash: '',
      txHeight: null,
      txHash: null,
      errorMessage: null,
      file: null,
      fromCid: null,
      toCid: null,
    });
    this.timeOut = null;
  };

  onClickInitStage = () => {
    this.clearState();
    this.setState({
      stage: STAGE_INIT,
    });
  };

  onClickClear = () => {
    this.setState({
      file: null,
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

  onClickBtnRank = async () => {
    const { addressLocalStor } = this.state;
    const { rankLink } = this.props;
    if (rankLink !== null) {
      await this.setState({
        contentHash: rankLink,
      });
    }

    if (addressLocalStor.keys === 'keplr' || addressLocalStor.keys === 'wallet') {
      this.onClickInitKeplr();
    }
  };

  onClickInit = () => {
    const { addressLocalStor } = this.state;

    if (addressLocalStor.keys === 'keplr' || addressLocalStor.keys === 'wallet') {
      this.onClickInitKeplr();
    }
  };

  render() {
    const { contentHash, stage, txHeight, txHash, errorMessage, file, addressLocalStor } =
      this.state;

    const { textBtn, placeholder, rankLink } = this.props;

    if (stage === STAGE_INIT && rankLink && rankLink !== null) {
      let keys = 'ledger';
      if (addressLocalStor !== null) {
        keys = addressLocalStor.keys;
      }
      return (
        <ActionBar>
          <ActionBarContentText>
            <ButtonImgText
              text={
                <Pane alignItems="center" display="flex">
                  Learn{' '}
                  <img
                    src={imgCyber}
                    alt="cyber"
                    style={{
                      width: 20,
                      height: 20,
                      marginLeft: '5px',
                      paddingTop: '2px',
                    }}
                  />
                </Pane>
              }
              onClick={() => this.onClickBtnRank()}
              img={keys === 'ledger' ? imgLedger : (keys === 'wallet' || process.env.IS_TAURI) ? imgWallet : imgKeplr}
            />
          </ActionBarContentText>
        </ActionBar>
      );
    }

    if (stage === STAGE_INIT) {
      return (
        <StartStageSearchActionBar
          textBtn={textBtn || 'Cyberlink'}
          keys={addressLocalStor !== null ? addressLocalStor.keys : false}
          onClickBtn={this.onClickInit}
          contentHash={file?.name || contentHash}
          onChangeInputContentHash={this.onChangeInput}
          inputOpenFileRef={this.inputOpenFileRef}
          showOpenFileDlg={this.showOpenFileDlg}
          onChangeInput={this.onFilePickerChange}
          onClickClear={this.onClickClear}
          file={file}
          placeholder={placeholder}
        />
      );
    }

    if (stage === STAGE_IPFS_HASH) {
      return (
        <ActionBar>
          <ActionBarContentText>
            adding content to IPFS <Dots big />
          </ActionBarContentText>
        </ActionBar>
      );
    }

    if (stage === STAGE_KEPLR_APPROVE) {
      return (
        <ActionBar>
          <ActionBarContentText>
            approve TX <Dots big />
          </ActionBarContentText>
        </ActionBar>
      );
    }

    if (stage === STAGE_READY) {
      return (
        <ActionBar>
          <ActionBarContentText>
            transaction generation <Dots big />
          </ActionBarContentText>
        </ActionBar>
      );
    }

    if (stage === STAGE_SUBMITTED || stage === STAGE_CONFIRMING) {
      return <TransactionSubmitted onClickBtnClose={this.onClickInitStage} />;
    }

    if (stage === STAGE_CONFIRMED) {
      return (
        <Confirmed
          txHash={txHash}
          txHeight={txHeight}
          onClickBtn={this.onClickInitStage}
          onClickBtnClose={this.onClickInitStage}
        />
      );
    }

    if (stage === STAGE_ERROR && errorMessage !== null) {
      return (
        <TransactionError
          errorMessage={errorMessage}
          onClickBtn={this.onClickInitStage}
          onClickBtnClose={this.onClickInitStage}
        />
      );
    }

    return null;
  }
}

const mapStateToProps = (store) => {
  return {
    defaultAccount: store.pocket.defaultAccount,
  };
};

export default withIpfsAndKeplr(connect(mapStateToProps)(ActionBarContainer));
