import { Coin, coin } from '@cosmjs/stargate';
import { DeliverTxResponse } from 'osmojs';
import { useState } from 'react';
import { ActionBar, Button } from 'src/components';
import { useEnergy } from 'src/pages/Energy/context/Energy.context';
import { useOsmosisSign } from 'src/pages/Energy/context/OsmosisSignerProvider';
import useTx from 'src/pages/Energy/hooks/useTx';
import { newSwapMessage } from 'src/pages/Energy/utils/swap';
import { useHub } from 'src/contexts/hub';
import { CHAIN_ID_OSMO } from 'src/pages/Energy/constants';
import BigNumber from 'bignumber.js';
import { useIbcHistory } from 'src/features/ibc-history/historyContext';
import { getNowUtcNumber } from 'src/utils/date';
import {
  resetEnergy,
  setIbcResult,
  setStatusOrder,
  setSwapResult,
} from 'src/pages/Energy/redux/energy.redux';
import { StatusOrder } from 'src/pages/Energy/redux/utils';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { StatusTx } from 'src/features/ibc-history/HistoriesItem';
import { getOsmoAssetByDenom } from 'src/pages/Energy/utils/utils';
import newIbcMessage from 'src/pages/Energy/utils/tranferIbc';

const coinFunc = (amount: string, denom: string): Coin => {
  return { denom, amount: new BigNumber(amount).toString(10) };
};

function newTokenSwapped(res: DeliverTxResponse) {
  const tokenSwapped = res.events.find((item) => item.type === 'token_swapped');

  const tokensOut = tokenSwapped?.attributes.find(
    (item) => item.key === 'tokens_out'
  );

  if (!tokensOut) {
    return undefined;
  }

  const value = tokensOut.value.split('ibc/');

  return coin(value[0], `ibc/${value[1]}`);
}

function ActionBarContainer() {
  const { refetch, energyPackageSwapRoutes } = useEnergy();
  const { address, signingClient } = useOsmosisSign();
  const { pingTxsIbc } = useIbcHistory();
  const { channels } = useHub();
  const dispatch = useAppDispatch();
  const { swapResult, selectPlan, statusOrder, ibcResult } = useAppSelector(
    (state) => state.energy
  );

  const findChannels = channels && channels[CHAIN_ID_OSMO];

  const selectPackage = energyPackageSwapRoutes?.find(
    (item) => item.keyPackage === selectPlan?.keyPackage
  );

  const { tx } = useTx();

  const [isSwapping, setIsSwapping] = useState(false);
  const [isIbcSending, setIsIbcSending] = useState(false);

  const onIbcTxs = async () => {
    if (!address || !signingClient || !findChannels) {
      return;
    }

    if (!swapResult) {
      return;
    }

    setIsIbcSending(true);

    const ibcMsg = newIbcMessage(swapResult.tokens, findChannels, address);

    console.log('ibcMsg', ibcMsg);

    const res = await tx([...ibcMsg]);

    setIsIbcSending(false);

    console.log('res', res);

    if (res.error) {
      console.log('{res.errorMsg}', res.errorMsg);
    } else if (res.isSuccess) {
      // ibcMsg.forEach((item) => {
      const { denom, amount } = swapResult.tokens[0];
      const { transactionHash } = res.response;
      const tokenSelect = getOsmoAssetByDenom(denom);
      const aliases = tokenSelect?.denom_units[0].aliases;
      const transferData = {
        txHash: transactionHash,
        address: ibcMsg[0].value.receiver,
        sourceChainId: findChannels.destination_chain_id,
        destChainId: findChannels.source_chain_id,
        sender: address,
        recipient: ibcMsg[0].value.receiver,
        createdAt: getNowUtcNumber(),
        amount: coinFunc(amount, aliases ? aliases[0] : denom),
      };

      pingTxsIbc(signingClient, transferData);

      dispatch(
        setIbcResult({ ibcHash: transactionHash, status: StatusTx.PENDING })
      );

      dispatch(setStatusOrder(StatusOrder.STATUS_IBC));
      // });

      console.log('IBC Send successful');

      refetch();
    }
  };

  const onBuyPackage = async () => {
    if (!address || !signingClient) {
      console.error('stargateClient undefined or address undefined.');
      return;
    }

    if (!selectPackage) {
      console.log('energyPackageSwapRoutes undefined');
      return;
    }
    console.log('address', address);

    setIsSwapping(true);

    const msgIn = newSwapMessage(selectPackage.swapInfo, address);
    console.log('msgIn', msgIn);

    const res = await tx([...msgIn]);
    setIsSwapping(false);

    console.log('res', res);

    if (res.error) {
      console.log('{res.errorMsg}', res.errorMsg);
    } else if (res.isSuccess) {
      console.log('Swap successful');
      if (res.response) {
        const tokenSwapped = newTokenSwapped(res.response);
        console.log('tokenSwapped', tokenSwapped);

        if (!tokenSwapped) {
          return;
        }
        dispatch(
          setSwapResult({
            swapTx: res.response.transactionHash,
            tokens: [tokenSwapped],
          })
        );
        dispatch(setStatusOrder(StatusOrder.SEND_IBC));
      }
      refetch();
    }
  };

  console.log('ibcResult', ibcResult);

  return (
    <ActionBar>
      {(statusOrder === StatusOrder.SELECT_PACK ||
        statusOrder === StatusOrder.SWAP) && (
        <Button text="change pack" onClick={() => dispatch(resetEnergy())} />
      )}
      {statusOrder === StatusOrder.SWAP && (
        <Button
          disabled={Boolean(swapResult?.tokens.length)}
          text="buy"
          pendingText="broadcastiong swap"
          onClick={onBuyPackage}
          pending={isSwapping}
        />
      )}
      {statusOrder === StatusOrder.SEND_IBC && (
        <Button
          disabled={!swapResult?.tokens.length}
          text="send to bostrom"
          onClick={onIbcTxs}
          pending={isIbcSending}
        />
      )}

      {(statusOrder === StatusOrder.STATUS_IBC ||
        statusOrder === StatusOrder.FINISH_IBC) && (
        <Button
          pending={statusOrder === StatusOrder.STATUS_IBC}
          pendingText="sending to bostrom"
          text="fuck google"
          onClick={() => dispatch(resetEnergy())}
        />
      )}
    </ActionBar>
  );
}

export default ActionBarContainer;
