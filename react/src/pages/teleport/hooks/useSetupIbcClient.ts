/* eslint-disable no-restricted-syntax */

import { GasPrice, SigningStargateClient } from '@cosmjs/stargate';
import { useEffect, useState } from 'react';
import { CHAIN_ID } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';

import networks from '../../../utils/networkListIbc';
import useGetBalancesIbc from './useGetBalancesIbc';

function useSetupIbcClient(denom, network) {
  const { signingClient, getSignerForChain } = useSigningClient();
  const [ibcClient, setIbcClient] = useState(null);
  const { balanceIbc, denomIbc, refresh: refreshBalanceIbc } = useGetBalancesIbc(ibcClient, denom);

  useEffect(() => {
    const createClient = async () => {
      setIbcClient(null);

      let client = null;
      if (network && network !== CHAIN_ID) {
        const networkConfig = networks[network];
        if (!networkConfig) return;

        const { rpc, prefix, chainId, gasPrice: gasPriceStr, coinMinimalDenom } = networkConfig;

        const offlineSigner = await getSignerForChain(chainId);
        if (!offlineSigner) return;

        const gasPrice = gasPriceStr
          ? GasPrice.fromString(gasPriceStr)
          : GasPrice.fromString(`0.025${coinMinimalDenom}`);

        const options = { prefix, gasPrice };
        client = await SigningStargateClient.connectWithSigner(rpc, offlineSigner, options);
      } else {
        client = signingClient;
      }
      setIbcClient(client);
    };
    createClient();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [network, signingClient]);

  return { ibcClient, balanceIbc, denomIbc, refreshBalanceIbc };
}

export default useSetupIbcClient;
