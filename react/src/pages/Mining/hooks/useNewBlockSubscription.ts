import { useEffect, useRef } from 'react';
import { useWebsockets } from 'src/websockets/context';

/**
 * Subscribes to Tendermint NewBlock events via the existing app WebSocket.
 * Calls `onNewBlock` whenever a new block is produced, enabling
 * event-driven refetching instead of fixed-interval polling.
 */
function useNewBlockSubscription(onNewBlock: () => void) {
  const { cyber } = useWebsockets();
  const subscribedRef = useRef(false);
  const callbackRef = useRef(onNewBlock);
  callbackRef.current = onNewBlock;

  // Subscribe once when WebSocket connects
  useEffect(() => {
    if (!cyber?.connected || subscribedRef.current) {
      return;
    }

    cyber.sendMessage({
      jsonrpc: '2.0',
      method: 'subscribe',
      id: 'mining-new-block',
      params: { query: "tm.event='NewBlock'" },
    });
    subscribedRef.current = true;
  }, [cyber?.connected, cyber?.sendMessage]);

  // Reset subscription flag when disconnected
  useEffect(() => {
    if (!cyber?.connected) {
      subscribedRef.current = false;
    }
  }, [cyber?.connected]);

  // React to incoming messages — fire callback on NewBlock events
  useEffect(() => {
    if (!cyber?.message) {
      return;
    }

    const msg = cyber.message;
    if (
      msg.id === 'mining-new-block' &&
      msg.result?.data?.type === 'tendermint/event/NewBlock'
    ) {
      callbackRef.current();
    }
  }, [cyber?.message]);

  return { connected: cyber?.connected ?? false };
}

export default useNewBlockSubscription;
