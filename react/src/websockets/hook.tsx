import { useEffect, useRef, useState } from 'react';

export type Socket = {
  connected: boolean;
  message: any | null;
  sendMessage: (message: object) => void;
  subscriptions: string[];
};

const RECONNECT_DELAY_MS = 3_000;
const MAX_RECONNECT_DELAY_MS = 30_000;

function useWebSocket(url: string): Socket {
  const [connected, setConnected] = useState(false);
  const [message, setMessage] = useState<Socket['message']>(null);

  const webSocketRef = useRef<WebSocket | null>(null);
  const subscriptions = useRef<string[]>([]);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectDelay = useRef(RECONNECT_DELAY_MS);
  const unmountedRef = useRef(false);

  useEffect(() => {
    unmountedRef.current = false;

    function connect() {
      if (unmountedRef.current) return;

      const ws = new WebSocket(url);
      webSocketRef.current = ws;

      ws.onopen = () => {
        reconnectDelay.current = RECONNECT_DELAY_MS;
        setConnected(true);

        // Re-send any previous subscriptions after reconnect
        for (const paramsStr of subscriptions.current) {
          ws.send(
            JSON.stringify({
              jsonrpc: '2.0',
              method: 'subscribe',
              id: `resub-${Date.now()}`,
              params: JSON.parse(paramsStr),
            })
          );
        }
      };

      ws.onmessage = (event) => {
        const newMessage = JSON.parse(event.data);
        setMessage(newMessage);
      };

      ws.onclose = () => {
        setConnected(false);
        if (unmountedRef.current) return;

        // Exponential backoff reconnect
        reconnectTimer.current = setTimeout(() => {
          reconnectDelay.current = Math.min(
            reconnectDelay.current * 1.5,
            MAX_RECONNECT_DELAY_MS
          );
          connect();
        }, reconnectDelay.current);
      };

      ws.onerror = () => {
        // onclose will fire after onerror, triggering reconnect
      };
    }

    connect();

    return () => {
      unmountedRef.current = true;
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
      }
      webSocketRef.current?.close();
    };
  }, [url]);

  function sendMessage(message: object) {
    if (!connected || !webSocketRef.current) {
      return;
    }

    if (message.method === 'subscribe') {
      const paramsStr = JSON.stringify(message.params);

      if (subscriptions.current.includes(paramsStr)) {
        return;
      }

      subscriptions.current.push(paramsStr);
    }

    webSocketRef.current.send(JSON.stringify(message));
  }

  return {
    connected,
    message,
    sendMessage,
    subscriptions: subscriptions.current,
  };
}

export default useWebSocket;
