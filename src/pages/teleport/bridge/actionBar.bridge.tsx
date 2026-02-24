import { Coin } from '@cosmjs/launchpad';
import { MsgTransferEncodeObject, SigningStargateClient } from '@cosmjs/stargate';
import BigNumber from 'bignumber.js';
import { MsgTransfer } from 'cosmjs-types/ibc/applications/transfer/v1/tx';
import Long from 'long';
import { useCallback, useState } from 'react';
import { DEFAULT_GAS_LIMITS } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import { Option } from 'src/types';
import { getNowUtcNumber } from 'src/utils/date';
import { ActionBar as ActionBarCenter, LinkWindow } from '../../../components';
import { useIbcHistory } from '../../../features/ibc-history/historyContext';
import { LEDGER } from '../../../utils/config';
import networks from '../../../utils/networkListIbc';
import { convertAmountReverce, fromBech32, trimString } from '../../../utils/utils';
import ActionBarPingTxs from '../components/actionBarPingTxs';
import { TxsType, TypeTxsT } from '../type';

const { STAGE_INIT, STAGE_ERROR, STAGE_SUBMITTED } = LEDGER;

const STAGE_CONFIRMED_IBC = 7.1;

const TIMEOUT_TIMESTAMP = 2 * 60 * 1000; // 2 min

const fee = {
  amount: [],
  gas: DEFAULT_GAS_LIMITS.toString(),
};

const coinFunc = (amount: number, denom: string): Coin => {
  return { denom, amount: new BigNumber(amount).toString(10) };
};

type Props = {
  tokenAmount: string;
  tokenSelect: string;
  networkB: string;
  updateFunc: () => void;
  isExceeded: boolean;
  typeTxs: TypeTxsT;
  ibcClient: null | SigningStargateClient;
  denomIbc: null | string;
  sourceChannel: string | null;
  coinDecimals: number;
};

function ActionBar({ stateActionBar }: { stateActionBar: Props }) {
  const { pingTxsIbc } = useIbcHistory();
  const { signingClient, signer } = useSigningClient();
  const [stage, setStage] = useState(STAGE_INIT);
  const [txHash, setTxHash] = useState<Option<string>>(undefined);
  const [txHashIbc, setTxHashIbc] = useState(null);
  const [linkIbcTxs, setLinkIbcTxs] = useState<Option<string>>(undefined);
  const [errorMessage, setErrorMessage] = useState<Option<string | JSX.Element>>(undefined);

  const {
    tokenAmount,
    tokenSelect,
    updateFunc,
    isExceeded,
    typeTxs,
    ibcClient,
    denomIbc,
    sourceChannel,
    networkB,
    coinDecimals,
  } = stateActionBar;

  const clearState = () => {
    setStage(STAGE_INIT);
    setTxHash(undefined);
    setErrorMessage(undefined);
    setTxHashIbc(null);
    setLinkIbcTxs(undefined);
  };

  const depositOnClick = useCallback(async () => {
    if (!ibcClient || !denomIbc || !signer) {
      return;
    }

    const [{ address }] = await ibcClient.signer.getAccounts();
    const [{ address: counterpartyAccount }] = await signer.getAccounts();
    const responseChainId = await ibcClient.getChainId();

    setStage(STAGE_SUBMITTED);

    const sourcePort = 'transfer';

    const timeoutTimestamp = Long.fromString(`${Date.now() + TIMEOUT_TIMESTAMP}000000`);

    const amount = convertAmountReverce(tokenAmount, coinDecimals);

    const transferAmount = coinFunc(amount, denomIbc);
    const msg: MsgTransferEncodeObject = {
      typeUrl: '/ibc.applications.transfer.v1.MsgTransfer',
      value: MsgTransfer.fromPartial({
        sourcePort,
        sourceChannel: sourceChannel || '',
        sender: address,
        receiver: counterpartyAccount,
        timeoutTimestamp: BigInt(timeoutTimestamp.toNumber()),
        token: transferAmount,
      }),
    };

    try {
      const response = await ibcClient.signAndBroadcast(address, [msg], 1.5, '');

      console.log('response', response);

      if (response.code === 0) {
        setTxHashIbc(response.transactionHash);
        setLinkIbcTxs(
          `${networks[responseChainId].explorerUrlToTx.replace(
            '{txHash}',
            response.transactionHash.toUpperCase()
          )}`
        );

        const transferData = {
          txHash: response.transactionHash,
          address: counterpartyAccount,
          sourceChainId: responseChainId,
          destChainId: networkB,
          sender: address,
          recipient: counterpartyAccount,
          createdAt: getNowUtcNumber(),
          amount: coinFunc(amount, tokenSelect),
        };
        pingTxsIbc(ibcClient, transferData);
        setStage(STAGE_CONFIRMED_IBC);
        // if (response.rawLog.length > 0) {
        //   parseRawLog(response.rawLog);
        // }
      } else {
        setTxHashIbc(null);
        setErrorMessage(response.rawLog.toString());
        setStage(STAGE_ERROR);
      }
    } catch (e) {
      console.error(`error: `, e);
      setTxHashIbc(null);
      setErrorMessage(e.toString());
      setStage(STAGE_ERROR);
    }

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    ibcClient,
    tokenAmount,
    denomIbc,
    networkB,
    coinDecimals,
    pingTxsIbc,
    signer,
    sourceChannel,
    tokenSelect,
  ]);

  const withdrawOnClick = useCallback(async () => {
    if (!signer || !signingClient) {
      return;
    }

    let prefix;
    setStage(STAGE_SUBMITTED);
    if (networks[networkB]) {
      prefix = networks[networkB].prefix;
    }
    const [{ address }] = await signer.getAccounts();
    const sourcePort = 'transfer';
    const counterpartyAccount = fromBech32(address, prefix);
    const timeoutTimestamp = Long.fromString(`${Date.now() + TIMEOUT_TIMESTAMP}000000`);

    const amount = convertAmountReverce(tokenAmount, coinDecimals);
    const transferAmount = coinFunc(amount, tokenSelect);
    const msg = {
      typeUrl: '/ibc.applications.transfer.v1.MsgTransfer',
      value: {
        sourcePort,
        sourceChannel,
        sender: address,
        receiver: counterpartyAccount,
        timeoutTimestamp: BigInt(timeoutTimestamp.toNumber()),
        token: transferAmount,
      },
    };
    try {
      const response = await signingClient.signAndBroadcast(address, [msg], fee, '');
      if (response.code === 0) {
        setTxHash(response.transactionHash);
        const ChainId = await signingClient.getChainId();
        const transferData = {
          txHash: response.transactionHash,
          address,
          sourceChainId: ChainId,
          destChainId: networkB,
          sender: address,
          recipient: counterpartyAccount,
          createdAt: getNowUtcNumber(),
          amount: transferAmount,
        };
        pingTxsIbc(signingClient, transferData);
      } else {
        setTxHash(undefined);
        setErrorMessage(response.rawLog.toString());
        setStage(STAGE_ERROR);
      }
    } catch (e) {
      console.error(`error: `, e);
      setTxHash(undefined);
      setErrorMessage(e.toString());
      setStage(STAGE_ERROR);
    }

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    tokenSelect,
    signer,
    tokenAmount,
    sourceChannel,
    networkB,
    coinDecimals,
    pingTxsIbc,
    signingClient,
  ]);

  const buttonConfigs = {
    [TxsType.Deposit]: {
      text: 'transfer',
      // onClick: () => addHistoriesItem(testItem),
      onClick: depositOnClick,
      disabled: isExceeded,
    },
    [TxsType.Withdraw]: {
      text: 'transfer',
      onClick: withdrawOnClick,
      disabled: isExceeded,
    },
  };

  if (stage === STAGE_INIT) {
    return <ActionBarCenter button={buttonConfigs[typeTxs]} />;
  }

  if (stage === STAGE_CONFIRMED_IBC) {
    return (
      <ActionBarCenter button={{ text: 'Fuck Google', onClick: clearState }}>
        <span>
          Transaction successful:{' '}
          <LinkWindow to={linkIbcTxs}>{trimString(txHashIbc, 6, 6)}</LinkWindow>
        </span>
      </ActionBarCenter>
    );
  }

  const stageActionBarStaps = {
    stage,
    setStage,
    updateFunc,
    clearState,
    txHash,
    errorMessageProps: errorMessage,
  };

  return <ActionBarPingTxs stageActionBarStaps={stageActionBarStaps} />;
}

export default ActionBar;
