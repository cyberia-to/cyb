import { useMutation } from '@tanstack/react-query';
import { useHub } from 'src/contexts/hub';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { CHAIN_ID_OSMO } from '../../constants';
import { useOsmosisSign } from '../../context/OsmosisSignerProvider';
import newIbcMessage from '../../utils/tranferIbc';
import useTx from '../useTx';
import { useEnergy } from '../../context/Energy.context';
import { setIbcResult, setStatusOrder } from '../../redux/energy.redux';
import { StatusOrder } from '../../redux/utils';

function useIbc() {
  const { refetch, energyPackageSwapRoutes } = useEnergy();
  const { channels } = useHub();
  const { address, signingClient } = useOsmosisSign();

  const dispatch = useAppDispatch();

  const { swapResult, selectPlan, statusOrder, ibcResult } = useAppSelector(
    (state) => state.energy
  );

  const { tx } = useTx();

  const findChannels = channels && channels[CHAIN_ID_OSMO];

  const { data, isLoading, error, mutate } = useMutation({
    mutationKey: [],
    mutationFn: async () => {
      if (!address || !signingClient || !findChannels || !swapResult) {
        return;
      }

      const ibcMsg = newIbcMessage(swapResult.tokens, findChannels, address);

      const res = await tx([...ibcMsg]);

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

        // pingTxsIbc(signingClient, transferData);

        dispatch(
          setIbcResult({ ibcHash: transactionHash, status: StatusTx.PENDING })
        );

        dispatch(setStatusOrder(StatusOrder.STATUS_IBC));
        // });

        console.log('IBC Send successful');

        refetch();
      }
    },
  });

  return null;
}

export default useIbc;
