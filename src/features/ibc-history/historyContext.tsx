/* eslint-disable */

import { Coin } from '@cosmjs/launchpad';
import { SigningStargateClient } from '@cosmjs/stargate';
import { parseRawLog } from '@cosmjs/stargate/build/logs';
import { SigningCyberClient } from '@cybercongress/cyber-js';
import { PromiseExtended } from 'dexie';
import React, { useCallback, useContext, useEffect, useState } from 'react';
import { useAppSelector } from 'src/redux/hooks';
import { RootState } from 'src/redux/store';
import { Option } from 'src/types';
import { AccountValue } from 'src/types/defaultAccount';
import networkList from 'src/utils/networkListIbc';
import { db as dbIbcHistory } from './db';
import { HistoriesItem, StatusTx } from './HistoriesItem';
import PollingStatusSubscription from './polling-status-subscription';
import TracerTx from './tx/TracerTx';
import parseEvents from './utils';

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
};

export const HistoryContext = React.createContext<HistoryContext>(valueContext);

export const useIbcHistory = () => {
  const context = useContext(HistoryContext);

  return context;
};

const _historiesItemsByAddress = (addressActive: AccountValue | null) => {
  if (addressActive) {
    return dbIbcHistory.historiesItems.where({ address: addressActive.bech32 }).toArray();
  }
  return [];
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
  const [ibcHistory, setIbcHistory] = useState<Option<HistoriesItem[]>>(undefined);
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  const [_update, setUpdate] = useState(0);
  const addressActive = defaultAccount.account?.cyber || undefined;

  function getBlockSubscriber(chainId: string): PollingStatusSubscription {
    if (!blockSubscriberMap.has(chainId)) {
      const chainInfo = findRpc(chainId);
      if (chainInfo) {
        blockSubscriberMap.set(chainId, new PollingStatusSubscription(chainInfo));
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
        new Date(blockTime).getTime() > Math.floor(parseInt(timeoutTimestamp, 10) / 1000000)
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
    if (item.status === StatusTx.COMPLETE || item.status === StatusTx.REFUNDED) {
      return item.status;
    }

    // First, always check if recv_packet already happened (packet was completed)
    const destRpc = findRpc(item.destChainId);
    if (destRpc) {
      try {
        const query = `recv_packet.packet_dst_channel='${item.destChannelId}' AND recv_packet.packet_sequence='${item.sequence}'`;
        const response = await fetch(
          `${destRpc}/tx_search?query=${encodeURIComponent(query)}&per_page=1`
        );
        if (response.ok) {
          const data = await response.json();
          if (data?.result?.total_count !== '0') {
            return StatusTx.COMPLETE;
          }
        }
      } catch {
        // ignore, continue with normal flow
      }
    }

    if (item.status === StatusTx.TIMEOUT) {
      const sourceChainId = findRpc(item.sourceChainId);
      if (!sourceChainId) return item.status;

      const txTracer = new TracerTx(sourceChainId, '/websocket');

      try {
        await txTracer.traceTx({
          'timeout_packet.packet_src_channel': item.sourceChannelId,
          'timeout_packet.packet_sequence': item.sequence,
        });
        txTracer.close();
        return StatusTx.REFUNDED;
      } catch {
        txTracer.close();
        return item.status;
      }
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
          // This it not the chain issue itself, just the issue from the frontend, it is impossible to ensure the network status entirely.
          // To reduce this problem, wait 30 seconds more for relay to complete (relay runs every 10s + indexing time).
          await new Promise((resolve) => {
            setTimeout(resolve, 30000);
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

    try {
      const result = await Promise.race(promises);

      if (timeoutUnsubscriber) {
        timeoutUnsubscriber();
      }

      txTracer.close();

      if (result) {
        return StatusTx.COMPLETE;
      }

      return StatusTx.TIMEOUT;
    } catch {
      if (timeoutUnsubscriber) {
        timeoutUnsubscriber();
      }
      txTracer.close();
      return item.status;
    }
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
  }, [addressActive]);

  const updateStatusByTxHash = async (txHash: string, status: StatusTx) => {
    const itemCollection = dbIbcHistory.historiesItems.where({ txHash });
    const itemByTxHash = await itemCollection.toArray();
    if (itemByTxHash && itemByTxHash[0].status !== status) {
      itemCollection.modify({ status });
    }
  };

  // Trace pending/timeout items on initial load (only when address changes)
  useEffect(() => {
    const tracePendingItems = async () => {
      if (addressActive) {
        const items = await dbIbcHistory.historiesItems
          .where({
            address: addressActive.bech32,
          })
          .toArray();

        items.forEach((item) => {
          if (item.status === StatusTx.PENDING || item.status === StatusTx.TIMEOUT) {
            traceHistoryStatus(item).then((newStatus) => {
              if (newStatus !== item.status) {
                updateStatusByTxHash(item.txHash, newStatus);
                setUpdate((i) => i + 1);
              }
            });
          }
        });
      }
    };
    tracePendingItems();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [addressActive, traceHistoryStatus, updateStatusByTxHash]);

  const pingTxsIbc = async (
    cliet: SigningStargateClient | SigningCyberClient,
    uncommitedTx: UncommitedTx
  ) => {
    const MAX_RETRIES = 40; // ~60 seconds
    let retryCount = 0;

    const ping = async () => {
      const response = await cliet.getTx(uncommitedTx.txHash);
      if (response) {
        const result = parseRawLog(response.rawLog);
        const dataFromEvent = parseEvents(result);
        if (dataFromEvent) {
          const itemHistories = { ...uncommitedTx, ...dataFromEvent };
          addHistoriesItem({
            ...itemHistories,
            status: StatusTx.PENDING,
          });
        }
        return;
      }
      retryCount += 1;
      if (retryCount >= MAX_RETRIES) {
        console.warn(`pingTxsIbc: max retries reached for tx ${uncommitedTx.txHash}`);
        return;
      }
      setTimeout(ping, 1500);
    };
    ping();
  };

  const addHistoriesItem = async (itemHistories: HistoriesItem) => {
    await dbIbcHistory.historiesItems.add(itemHistories);
    setUpdate((item) => item + 1);

    // Automatically start tracing pending items
    if (itemHistories.status === StatusTx.PENDING) {
      traceHistoryStatus(itemHistories).then((newStatus) => {
        if (newStatus !== itemHistories.status) {
          updateStatusByTxHash(itemHistories.txHash, newStatus);
          setUpdate((item) => item + 1);
        }
      });
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
      }}
    >
      {children}
    </HistoryContext.Provider>
  );
}

export default HistoryContextProvider;
