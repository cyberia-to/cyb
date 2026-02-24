import React, { useContext, useMemo } from 'react';
import { useChannels, useNetworks, useTokens } from 'src/hooks/useHub';
import { Option } from 'src/types';
import { ObjectKey } from 'src/types/data';
import { Channel, Network, Token } from 'src/types/hub';

type HubProviderContextType = {
  channels: Option<ObjectKey<Channel>>;
  tokens: Option<ObjectKey<Token>>;
  networks: Option<ObjectKey<Network>>;
};

const valueContext = {
  channels: undefined,
  tokens: undefined,
  networks: undefined,
};

const HubProviderContext = React.createContext<HubProviderContextType>(valueContext);

export function useHub() {
  return useContext(HubProviderContext);
}

function HubProvider({ children }: { children: React.ReactNode }) {
  const { networks } = useNetworks();
  const { tokens } = useTokens();
  const { channels } = useChannels();

  const valueMemo = useMemo(
    () => ({
      networks,
      tokens,
      channels,
    }),
    [networks, tokens, channels]
  );

  return <HubProviderContext.Provider value={valueMemo}>{children}</HubProviderContext.Provider>;
}

export default HubProvider;
