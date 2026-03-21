import { useEffect, useRef, useState } from 'react';

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
