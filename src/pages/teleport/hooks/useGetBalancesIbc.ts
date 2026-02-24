/* eslint-disable no-restricted-syntax */

import { Sha256 } from '@cosmjs/crypto';
import { SigningStargateClient } from '@cosmjs/stargate';
import { useEffect, useState } from 'react';
import { CHAIN_ID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { toHex } from 'src/utils/encoding';
import networkList from '../../../utils/networkListIbc';
import useSubscribersBlokIbc from './useSubscribersBlokIbc';

// src/utils have same
const sha256 = (data) => {
  return new Uint8Array(new Sha256().update(data).digest());
};

const ibcDenom = (paths, coinMinimalDenom) => {
  const prefixes = [];
  for (const path of paths) {
    prefixes.push(`${path.portId}/${path.channelId}`);
  }
  const prefix = prefixes.join('/');
  const denom = `${prefix}/${coinMinimalDenom}`;

  return `ibc/${toHex(sha256(denom)).toUpperCase()}`;
};

function useGetBalancesIbc(client: SigningStargateClient, denom) {
  const { ibcDenoms: ibcDataDenom } = useIbcDenom();
  const [balanceIbc, setBalanceIbc] = useState(null);
  const [denomIbc, setDenomIbc] = useState(null);
  const [error, setError] = useState(null);
  const { blockInfo } = useSubscribersBlokIbc(client);
  const [_update, setUpdate] = useState(0);

  const getBalanceIbc = async () => {
    if (client && denom) {
      const responseChainId = client.signer.chainId;
      if (responseChainId !== CHAIN_ID) {
        let coinMinimalDenom = null;
        if (denom.includes('ibc') && ibcDataDenom) {
          coinMinimalDenom = ibcDataDenom[denom].baseDenom;
        } else {
          coinMinimalDenom = ibcDenom(
            [
              {
                portId: 'transfer',
                channelId: networkList[responseChainId].destChannelId,
              },
            ],
            denom
          );
        }

        try {
          const [{ address }] = await client.signer.getAccounts();
          const responseBalance = await client.queryClient.bank.balance(address, coinMinimalDenom);

          return {
            balance: { [coinMinimalDenom]: responseBalance.amount },
            denom: coinMinimalDenom,
          };
        } catch (error) {
          console.error(error);
          setError(error);
          return undefined;
        }
      }
    }
    return undefined;
  };

  useEffect(() => {
    const getBalance = async () => {
      setBalanceIbc(null);
      setDenomIbc(null);

      const response = await getBalanceIbc();

      if (response) {
        const { balance, denom } = response;
        setDenomIbc(denom);
        setBalanceIbc(balance);
      }
    };
    getBalance();
  }, [getBalanceIbc]);

  useEffect(() => {
    const updateBalanceIbc = async () => {
      const result = await getBalanceIbc();
      if (result && balanceIbc) {
        const { balance } = result;
        const key = Object.keys(balance)[0];
        if (balanceIbc[key] && Number(balance[key]) !== Number(balanceIbc[key])) {
          setBalanceIbc(balance);
        }
      }
    };
    updateBalanceIbc();
  }, [balanceIbc, getBalanceIbc]);

  useEffect(() => {
    if (client) {
      const responseChainId = client.signer.chainId;

      if (blockInfo?.chainId === responseChainId) {
        setUpdate((item) => item + 1);
      }
    }
  }, [blockInfo, client]);

  return { balanceIbc, denomIbc, error };
}

export default useGetBalancesIbc;
