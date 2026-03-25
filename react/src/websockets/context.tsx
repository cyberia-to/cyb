import React, { useMemo } from 'react';
import { WEBSOCKET_URL } from 'src/constants/config';
import useWebSocket, { Socket } from './hook';

const WebsocketsContext = React.createContext<{ cyber: Socket | null }>({
  cyber: null,
});

export function useWebsockets() {
  return React.useContext(WebsocketsContext);
}

function WebsocketsProvider({ children }: { children: React.ReactNode }) {
  const cyberWebsocket = useWebSocket(WEBSOCKET_URL);

  const value = useMemo(() => {
    return {
      cyber: cyberWebsocket,
      // may be added more
    };
  }, [cyberWebsocket]);

  return <WebsocketsContext.Provider value={value}>{children}</WebsocketsContext.Provider>;
}

export default WebsocketsProvider;
