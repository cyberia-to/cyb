/* eslint-disable camelcase */
/* eslint-disable no-restricted-syntax */

import { Decimal } from '@cosmjs/math';
import { GasPrice, SigningStargateClient } from '@cosmjs/stargate';
import { useEffect, useState } from 'react';
import { CHAIN_ID } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import { getKeplr } from 'src/utils/keplrUtils';
import { CybOfflineSigner, getOfflineSigner } from 'src/utils/offlineSigner';
import { OfflineAminoSigner } from '@keplr-wallet/types';
import { chains, assets } from 'chain-registry';
import {
  getGasPriceRangesFromChain,
  getChainByChainName,
} from '@chain-registry/utils';
import { getMnemonic } from 'src/utils/utils';

import networks from '../../../utils/networkListIbc';
import useGetBalancesIbc from './useGetBalancesIbc';

function useSetupIbcClient(denom, network) {
  const { signingClient } = useSigningClient();
  const [ibcClient, setIbcClient] = useState(null);
  const { balanceIbc, denomIbc, refresh: refreshBalanceIbc } = useGetBalancesIbc(ibcClient, denom);

  useEffect(() => {
    const createClient = async () => {
      setIbcClient(null);

      let client = null;
      if (network && network !== CHAIN_ID) {
        const { rpc, prefix, sourceChainId, chainId } = networks[network];
        const keplr = await getKeplr();
        let offlineSigner: OfflineAminoSigner | CybOfflineSigner | null = null;

        if (keplr) {
          await keplr.enable(chainId);
          offlineSigner = (await keplr.getOfflineSignerAuto(
            chainId
          )) as OfflineAminoSigner;
        } else {
          const mnemonics = getMnemonic();
          if (mnemonics) {
            offlineSigner = await getOfflineSigner(mnemonics, network);
          }
        }

        const chain = getChainByChainName(chains, sourceChainId);
        if (chain) {
          const gasPriceRanges = getGasPriceRangesFromChain(chain!);
          const assetList = assets.find(
            ({ chain_name }) => chain_name === sourceChainId
          );
          const minimalDenom = assetList!.assets[0].base;

          const GasPriceA = new GasPrice(
            Decimal.fromUserInput(gasPriceRanges?.low || '0', 3),
            minimalDenom
          );
          console.log({ GasPriceA, minimalDenom });

          const options = {
            prefix,
            gasPrice: GasPriceA,
          };
          client = await SigningStargateClient.connectWithSigner(
            rpc,
            offlineSigner,
            options
          );
        }
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
