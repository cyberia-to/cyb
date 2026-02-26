import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

export type Socket = {
  connected: boolean;
  message: any | null;
  sendMessage: (message: object) => void;
  subscriptions: string[];
};

function useWebSocket(url: string): Socket {
  const [connected, setConnected] = useState(false);
  const [message, setMessage] = useState<Socket['message']>(null);

  const webSocketRef = useRef<WebSocket | null>(null);
  const subscriptions = useRef<string[]>([]);

  useEffect(() => {
    webSocketRef.current = new WebSocket(url);

    webSocketRef.current.onopen = () => {
      setConnected(true);
    };

    webSocketRef.current.onmessage = (event) => {
      const newMessage = JSON.parse(event.data);
      setMessage(newMessage);
    };

    webSocketRef.current.onclose = () => {
      setConnected(false);
    };

    return () => {
      webSocketRef.current?.close();
    };
  }, [url]);

  const sendMessage = useCallback((message: object) => {
    if (!webSocketRef.current || webSocketRef.current.readyState !== WebSocket.OPEN) {
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
  }, []);

  return useMemo(() => ({
    connected,
    message,
    sendMessage,
    subscriptions: subscriptions.current,
  }), [connected, message, sendMessage]);
}

export default useWebSocket;
