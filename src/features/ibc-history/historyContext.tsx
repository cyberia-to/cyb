/* eslint-disable */
import React, { useContext, useState, useEffect, useCallback } from 'react';
import { db as dbIbcHistory } from './db';
import { HistoriesItem, StatusTx } from './HistoriesItem';
import { RootState } from 'src/redux/store';
import { AccountValue } from 'src/types/defaultAccount';
import { Coin } from '@cosmjs/launchpad';
import { parseRawLog } from '@cosmjs/stargate/build/logs';
import parseEvents from './utils';
import { SigningStargateClient } from '@cosmjs/stargate';
import { SigningCyberClient } from '@cybercongress/cyber-js';
import { Option } from 'src/types';
import { PromiseExtended } from 'dexie';
import TracerTx from './tx/TracerTx';
import networkList from 'src/utils/networkListIbc';
import PollingStatusSubscription from './polling-status-subscription';
import { useAppSelector } from 'src/redux/hooks';

const findRpc = (chainId: string): Option<string> => {
  if (networkList[chainId]) {
    return networkList[chainId].rpc;
  }

  return undefined;
};

type HistoryContext = {
  ibcHistory: Option<HistoriesItem[]>;
  changeHistory: () => void;
  addHistoriesItem: (itemHistories: HistoriesItem) => void;
  pingTxsIbc: (
    cliet: SigningStargateClient | SigningCyberClient,
    uncommitedTx: UncommitedTx
  ) => void;
  useGetHistoriesItems: () => Option<PromiseExtended<HistoriesItem[]>>;
  updateStatusByTxHash: (txHash: string, status: StatusTx) => void;
  getItemByTxHash: (txHash: string) => Promise<HistoriesItem[]>;
  traceHistoryStatus: (item: HistoriesItem) => Promise<StatusTx>;
};

const valueContext = {
  ibcHistory: undefined,
  changeHistory: () => {},
  addHistoriesItem: () => {},
  pingTxsIbc: () => {},
  useGetHistoriesItems: () => {},
  updateStatusByTxHash: () => {},
  traceHistoryStatus: () => {},
  getItemByTxHash: () => {},
};

export const HistoryContext = React.createContext<HistoryContext>(valueContext);

export const useIbcHistory = () => {
  const context = useContext(HistoryContext);

  return context;
};

type UncommitedTx = {
  txHash: string;
  address: string;
  sourceChainId: string;
  destChainId: string;
  sender: string;
  recipient: string;
  createdAt: number;
  amount: Coin;
};

const blockSubscriberMap: Map<string, PollingStatusSubscription> = new Map();

function HistoryContextProvider({ children }: { children: React.ReactNode }) {
  const [ibcHistory, setIbcHistory] =
    useState<Option<HistoriesItem[]>>(undefined);
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  const [update, setUpdate] = useState(0);
  const addressActive = defaultAccount.account?.cyber || undefined;

  function getBlockSubscriber(chainId: string): PollingStatusSubscription {
    if (!blockSubscriberMap.has(chainId)) {
      const chainInfo = findRpc(chainId);
      if (chainInfo) {
        blockSubscriberMap.set(
          chainId,
          new PollingStatusSubscription(chainInfo)
        );
      }
    }

    // eslint-disable-next-line
    return blockSubscriberMap.get(chainId)!;
  }

  function traceTimeoutTimestamp(
    statusSubscriber: PollingStatusSubscription,
    timeoutTimestamp: string
  ): {
    unsubscriber: () => void;
    promise: Promise<void>;
  } {
    let resolver: (value: PromiseLike<void> | void) => void;
    const promise = new Promise<void>((resolve) => {
      resolver = resolve;
    });
    const unsubscriber = statusSubscriber.subscribe((data) => {
      const blockTime = data?.result?.sync_info?.latest_block_time;
      if (
        blockTime &&
        new Date(blockTime).getTime() >
          Math.floor(parseInt(timeoutTimestamp) / 1000000)
      ) {
        resolver();
        return;
      }
    });

    return {
      unsubscriber,
      promise,
    };
  }

  const traceHistoryStatus = async (item: HistoriesItem): Promise<StatusTx> => {
    if (
      item.status === StatusTx.COMPLETE ||
      item.status === StatusTx.REFUNDED
    ) {
      return item.status;
    }

    if (item.status === StatusTx.TIMEOUT) {
      const sourceChainId = findRpc(item.sourceChainId);
      if (!sourceChainId) return item.status;

      const txTracer = new TracerTx(sourceChainId, '/websocket');

      await txTracer.traceTx({
        'timeout_packet.packet_src_channel': item.sourceChannelId,
        'timeout_packet.packet_sequence': item.sequence,
      });

      txTracer.close();
      return StatusTx.REFUNDED;
    }

    const blockSubscriber = getBlockSubscriber(item.destChainId);

    let timeoutUnsubscriber: (() => void) | undefined;

    const promises: Promise<any>[] = [];

    if (item.timeoutTimestamp && item.timeoutTimestamp !== '0') {
      promises.push(
        (async () => {
          const { promise, unsubscriber } = traceTimeoutTimestamp(
            blockSubscriber,
            // eslint-disable-next-line
            item.timeoutTimestamp!
          );
          timeoutUnsubscriber = unsubscriber;
          await promise;

          // Even though the block is reached to the timeout height,
          // the receiving packet event could be delivered before the block timeout if the network connection is unstable.
          // This it not the chain issue itself, jsut the issue from the frontend, it it impossible to ensure the network status entirely.
          // To reduce this problem, just wait 10 second more even if the block is reached to the timeout height.
          await new Promise((resolve) => {
            setTimeout(resolve, 10000);
          });
        })()
      );
    }

    const destChainId = findRpc(item.destChainId);

    if (!destChainId) return item.status;

    const txTracer = new TracerTx(destChainId, '/websocket');

    promises.push(
      txTracer.traceTx({
        'recv_packet.packet_dst_channel': item.destChannelId,
        'recv_packet.packet_sequence': item.sequence,
      })
    );

    const result = await Promise.race(promises);

    if (timeoutUnsubscriber) {
      timeoutUnsubscriber();
    }

    txTracer.close();

    if (result) {
      return StatusTx.COMPLETE;
    }

    return StatusTx.TIMEOUT;
  };

  const useGetHistoriesItems = useCallback(() => {
    if (addressActive) {
      return dbIbcHistory.historiesItems
        .where({
          address: addressActive.bech32,
        })
        .toArray();
    }
    return undefined;
  }, [addressActive]);

  useEffect(() => {
    const getItem = async () => {
      if (addressActive) {
        const response = await dbIbcHistory.historiesItems
          .where({
            address: addressActive.bech32,
          })
          .toArray();
        if (response) {
          setIbcHistory(response.reverse());
        }
      }
    };
    getItem();
  }, [addressActive, update]);

  const getItemByTxHash = async (txHash: string) => {
    return dbIbcHistory.historiesItems
      .where({
        txHash,
      })
      .toArray();
  };

  const pingTxsIbc = async (
    cliet: SigningStargateClient | SigningCyberClient,
    uncommitedTx: UncommitedTx
  ) => {
    const ping = async () => {
      const response = await cliet.getTx(uncommitedTx.txHash);
      if (response && response.events) {
        const dataFromEvent = parseEvents(response.events);
        if (dataFromEvent) {
          const itemHistories = { ...uncommitedTx, ...dataFromEvent };
          addHistoriesItem({
            ...itemHistories,
            status: StatusTx.PENDING,
          });
        }
        return;
      }
      setTimeout(ping, 1500);
    };
    ping();
  };

  const addHistoriesItem = (itemHistories: HistoriesItem) => {
    dbIbcHistory.historiesItems.add(itemHistories);
    setUpdate((item) => item + 1);
  };

  const updateStatusByTxHash = async (txHash: string, status: StatusTx) => {
    const itemCollection = dbIbcHistory.historiesItems.where({ txHash });
    const itemByTxHash = await itemCollection.toArray();
    if (itemByTxHash && itemByTxHash[0].status !== status) {
      itemCollection.modify({ status });
    }
  };

  const changeHistory = () => {
    // console.log('history', history);
    // setValue((item) => ({ ...item, history: { ...item.history, history } }));
  };

  return (
    <HistoryContext.Provider
      value={{
        ibcHistory,
        changeHistory,
        addHistoriesItem,
        pingTxsIbc,
        useGetHistoriesItems,
        updateStatusByTxHash,
        traceHistoryStatus,
        getItemByTxHash,
      }}
    >
      {children}
    </HistoryContext.Provider>
  );
}

export default HistoryContextProvider;
