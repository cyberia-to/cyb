import { Decimal } from '@cosmjs/math';
import { GasPrice } from '@cosmjs/stargate';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useChannels } from 'src/hooks/useHub';
import loadConnections from 'src/services/relayer/loadConnections';
import relay from 'src/services/relayer/relay';
import { ObjectKey } from 'src/types/data';
import { Channel } from 'src/types/hub';
import { getKeplr } from 'src/utils/keplrUtils';
import networkList from 'src/utils/networkListIbc';

const RelayerContext = React.createContext<{
  channels: undefined | ObjectKey<Channel>;
  relayerLog: any[];
  isRelaying: boolean;
  selectChain: string;
  setSelectChain: (key: string) => void;
  stop: () => void;
}>({
  channels: undefined,
  relayerLog: [],
  isRelaying: false,
  selectChain: '',
  setSelectChain: () => undefined,
  stop: () => undefined,
});

function findNetwork(chainId: string) {
  return networkList[chainId];
}

const logMap = ['log', 'info', 'error', 'warn', 'verbose', 'debug'];

export const useRelayer = () => React.useContext(RelayerContext);

function RelayerContextProvider({ children }: { children: React.ReactNode }) {
  const { channels } = useChannels();

  const stopFn = useRef<() => void>();

  const [selectChain, setSelectChain] = useState('');
  const [isRelaying, setIsRelaying] = useState(false);
  const [relayerLog, setRelayerLog] = useState<string[]>([]);

  const logger = () => {
    return logMap.reduce(
      (obj, item) => ({
        ...obj,
        [item]: (msg: string) => {
          setRelayerLog((state) => [...state, `${item}:  ${msg}`]);
        },
      }),
      {}
    );
  };

  useEffect(() => {
    return () => {
      stopFn.current?.();
    };
  }, []);

  useEffect(() => {
    (async () => {
      if (selectChain.length > 0) {
        setIsRelaying(true);
        const keplrWindow = await getKeplr();

        stopFn.current?.();

        if (!channels || !keplrWindow) {
          return;
        }

        const chainInfos = await keplrWindow.getChainInfosWithoutEndpoints();

        const { source_chain_id: chainIdA, destination_chain_id: chainIdB } = channels[selectChain];

        const chainInfoA = chainInfos.find((item) => item.chainId === chainIdA);
        const chainInfoB = chainInfos.find((item) => item.chainId === chainIdB);

        if (!chainInfoA || !chainInfoB) {
          return;
        }

        const feeCurrenciesA = chainInfoA.feeCurrencies[0];
        const feeCurrenciesB = chainInfoB.feeCurrencies[0];

        const GasPriceA = new GasPrice(
          Decimal.fromUserInput(feeCurrenciesA.gasPriceStep?.average.toString() || '0', 3),
          feeCurrenciesA?.coinMinimalDenom
        );

        const GasPriceB = new GasPrice(
          Decimal.fromUserInput(feeCurrenciesB.gasPriceStep?.average.toString() || '0', 3),
          feeCurrenciesB?.coinMinimalDenom
        );

        const { rpc: rpcA } = findNetwork(chainIdA);
        const { rpc: rpcB } = findNetwork(chainIdB);

        const signerA = await keplrWindow.getOfflineSignerAuto(chainIdA);
        const signerB = await keplrWindow.getOfflineSignerAuto(chainIdB);

        const [{ address: addressSignerA }] = await signerA.getAccounts();
        const [{ address: addressSignerB }] = await signerB.getAccounts();

        const cxns = await loadConnections(channels[selectChain]);

        if (!cxns.length) {
          return;
        }

        console.debug('cxns', cxns);

        const [{ cxnA, cxnB }] = cxns;

        (async () => {
          const config = await relay(
            signerA,
            signerB,
            rpcA,
            rpcB,
            addressSignerA,
            addressSignerB,
            GasPriceA,
            GasPriceB,
            cxnA,
            cxnB,
            addressSignerA,
            addressSignerB,
            0,
            0,
            logger()
          );

          stopFn.current = () => {
            config.stop();
            setIsRelaying(false);
            setSelectChain('');
            setRelayerLog([]);
            stopFn.current = undefined;
          };
        })();
      }
    })();

    return () => {
      stopFn.current?.();
    };
  }, [selectChain, channels, logger]);

  const stop = () => {
    stopFn.current?.();
  };

  const contextValue = useMemo(
    () => ({
      channels,
      relayerLog,
      isRelaying,
      selectChain,
      setSelectChain,
      stop,
    }),
    [channels, relayerLog, isRelaying, selectChain, stop]
  );

  return <RelayerContext.Provider value={contextValue}>{children}</RelayerContext.Provider>;
}

export default RelayerContextProvider;
