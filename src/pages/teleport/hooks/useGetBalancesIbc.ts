/* eslint-disable no-restricted-syntax */

import { Sha256 } from '@cosmjs/crypto';
import { SigningStargateClient } from '@cosmjs/stargate';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { CHAIN_ID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { toHex } from 'src/utils/encoding';
import networkList from '../../../utils/networkListIbc';
import useSubscribersBlokIbc from './useSubscribersBlokIbc';

const encoder = new TextEncoder();

const sha256 = (data: string) => {
  return new Uint8Array(new Sha256().update(encoder.encode(data)).digest());
};

const ibcDenom = (paths, coinMinimalDenom) => {
  const prefixes = [];
  for (const path of paths) {
    prefixes.push(`${path.portId}/${path.channelId}`);
  }
  const prefix = prefixes.join('/');
  const denomPath = `${prefix}/${coinMinimalDenom}`;

  return `ibc/${toHex(sha256(denomPath)).toUpperCase()}`;
};

function useGetBalancesIbc(client: SigningStargateClient, denom) {
  const { ibcDenoms: ibcDataDenom } = useIbcDenom();
  const [balanceIbc, setBalanceIbc] = useState(null);
  const [denomIbc, setDenomIbc] = useState(null);
  const [error, setError] = useState(null);
  const { blockInfo } = useSubscribersBlokIbc(client);
  const [_update, setUpdate] = useState(0);

  const coinMinimalDenom = useMemo(() => {
    if (!client || !denom) return null;
    const responseChainId = client.signer.chainId;
    if (responseChainId === CHAIN_ID) return null;

    if (denom.includes('ibc') && ibcDataDenom && ibcDataDenom[denom]) {
      return ibcDataDenom[denom].baseDenom;
    }
    const network = networkList[responseChainId];
    if (!network?.destChannelId) return null;
    return ibcDenom(
      [{ portId: 'transfer', channelId: network.destChannelId }],
      denom
    );
  }, [client, denom, ibcDataDenom]);

  const fetchBalance = useCallback(async () => {
    if (!client || !coinMinimalDenom) return undefined;
    try {
      const [{ address }] = await client.signer.getAccounts();
      const responseBalance = await client.queryClient.bank.balance(address, coinMinimalDenom);
      return { [coinMinimalDenom]: responseBalance.amount };
    } catch (err) {
      console.error(err);
      setError(err);
      return undefined;
    }
  }, [client, coinMinimalDenom]);

  // Initial fetch when client/denom changes
  useEffect(() => {
    const getBalance = async () => {
      setBalanceIbc(null);
      setDenomIbc(null);

      const balance = await fetchBalance();
      if (balance) {
        setDenomIbc(coinMinimalDenom);
        setBalanceIbc(balance);
      }
    };
    getBalance();
  }, [fetchBalance, coinMinimalDenom]);

  // Refetch on new blocks or manual refresh
  useEffect(() => {
    if (_update === 0) return;
    const updateBalance = async () => {
      const balance = await fetchBalance();
      if (balance) {
        setBalanceIbc(balance);
      }
    };
    updateBalance();
  }, [_update, fetchBalance]);

  // Increment _update on new blocks matching our chain
  useEffect(() => {
    if (client && blockInfo) {
      const responseChainId = client.signer.chainId;
      if (blockInfo.chainId === responseChainId) {
        setUpdate((item) => item + 1);
      }
    }
  }, [blockInfo, client]);

  // Polling: after refresh() is called, poll every 10s for ~5 min until balance changes
  const pollingRef = useRef<ReturnType<typeof setInterval>>();
  const snapshotRef = useRef<string | null>(null);

  const stopPolling = useCallback(() => {
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = undefined;
    }
  }, []);

  const refresh = useCallback(() => {
    setUpdate((item) => item + 1);

    // Snapshot current balance to detect change
    if (balanceIbc) {
      const key = Object.keys(balanceIbc)[0];
      snapshotRef.current = key ? balanceIbc[key] : null;
    }

    stopPolling();
    let count = 0;
    pollingRef.current = setInterval(async () => {
      count++;
      const balance = await fetchBalance();
      if (balance) {
        const key = Object.keys(balance)[0];
        const newAmount = key ? balance[key] : null;
        if (newAmount !== snapshotRef.current) {
          setBalanceIbc(balance);
          stopPolling();
          return;
        }
      }
      if (count >= 30) {
        stopPolling();
      }
    }, 10000);
  }, [balanceIbc, fetchBalance, stopPolling]);

  // Cleanup polling on unmount
  useEffect(() => () => stopPolling(), [stopPolling]);

  return { balanceIbc, denomIbc, error, refresh };
}

export default useGetBalancesIbc;
