import { useState, useCallback } from 'react';
import BigNumber from 'bignumber.js';
import useSetActiveAddress from 'src/hooks/useSetActiveAddress';
import { useSigningClient } from 'src/contexts/signerClient';
import { Option } from 'src/types';
import { Coin } from '@cosmjs/launchpad';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { RootState } from 'src/redux/store';
import { useNavigate } from 'react-router-dom';
import {
  Params,
  Pool,
} from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { useAppSelector } from 'src/redux/hooks';
import { Account, ActionBar as ActionBarCenter } from '../../../components';
import { LEDGER } from '../../../utils/config';
import { convertAmountReverce, convertAmount } from '../../../utils/utils';

import ActionBarPingTxs from '../components/actionBarPingTxs';
import { sortReserveCoinDenoms } from './utils';

const POOL_TYPE_INDEX = 1;

const { STAGE_INIT, STAGE_ERROR, STAGE_SUBMITTED } = LEDGER;

const coinFunc = (amount: number, denom: string): Coin => {
  return { denom, amount: new BigNumber(amount).toString(10) };
};

type Props = {
  tokenAAmount: string;
  tokenA: string;
  tokenB: string;
  params: undefined | Params;
  selectedPool: Pool | undefined;
  updateFunc: () => void;
  isExceeded: boolean;
  swapPrice: number;
  poolPrice: number;
};

function ActionBar({ stateActionBar }: { stateActionBar: Props }) {
  const navigate = useNavigate();
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const { signingClient, signer } = useSigningClient();
  const { tracesDenom } = useIbcDenom();
  const [stage, setStage] = useState(STAGE_INIT);
  const [txHash, setTxHash] = useState<Option<string>>(undefined);
  const [errorMessage, setErrorMessage] =
    useState<Option<string | JSX.Element>>(undefined);

  const {
    tokenAAmount,
    tokenA,
    tokenB,
    params,
    selectedPool,
    updateFunc,
    isExceeded,
    swapPrice,
    poolPrice,
  } = stateActionBar;

  const swapWithinBatch = async () => {
    if (signer && selectedPool && params && signingClient && tracesDenom) {
      const [{ address }] = await signer.getAccounts();

      const [{ coinDecimals: coinDecimalsA }] = tracesDenom(tokenA);

      const amountTokenA = convertAmountReverce(tokenAAmount, coinDecimalsA);

      setStage(STAGE_SUBMITTED);
      const offerCoinFee = coinFunc(
        Math.ceil(
          amountTokenA * convertAmount(parseFloat(params.swapFeeRate), 18) * 0.5
        ),
        tokenA
      );

      const offerCoin = coinFunc(amountTokenA, tokenA);
      const demandCoinDenom = tokenB;

      const exp = new BigNumber(10).pow(18).toString();
      const convertSwapPrice = new BigNumber(swapPrice)
        .multipliedBy(exp)
        .dp(0, BigNumber.ROUND_FLOOR)
        .toString(10);
      if (addressActive !== null && addressActive.bech32 === address) {
        try {
          const response = await signingClient.swapWithinBatch(
            address,
            selectedPool.id,
            POOL_TYPE_INDEX,
            offerCoin,
            demandCoinDenom,
            offerCoinFee,
            convertSwapPrice,
            'auto'
          );

          if (response.code === 0) {
            setTxHash(response.transactionHash);
          } else {
            setTxHash(undefined);
            setErrorMessage(response.rawLog.toString());
            setStage(STAGE_ERROR);
          }
        } catch (error) {
          setTxHash(undefined);
          setErrorMessage(error.toString());
          setStage(STAGE_ERROR);
        }
      } else {
        setErrorMessage(
          <span>
            Add address <Account margin="0 5px" address={address} /> to your
            pocket or make active{' '}
          </span>
        );
        setStage(STAGE_ERROR);
      }
    }
  };

  const clearState = () => {
    setStage(STAGE_INIT);
    setTxHash(undefined);
    setErrorMessage(undefined);
  };

  const createPool = useCallback(() => {
    const sortCoin = sortReserveCoinDenoms(tokenA, tokenB);
    navigate(`/warp/create-pool?from=${sortCoin[0]}&to=${sortCoin[1]}`);
  }, [tokenA, tokenB, navigate]);

  if (!selectedPool && stage === STAGE_INIT) {
    return (
      <ActionBarCenter
        button={{
          text: 'Create pool',
          onClick: createPool,
        }}
      />
    );
  }

  if (stage === STAGE_INIT) {
    return (
      <ActionBarCenter
        button={{
          text: 'Swap',
          onClick: swapWithinBatch,
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
