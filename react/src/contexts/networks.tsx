import React, { useContext, useEffect, useMemo, useState } from 'react';
import defaultNetworks from 'src/constants/defaultNetworks';
import { Option } from 'src/types';
import { NetworksList } from 'src/types/networks';

type NetworksContext = {
  networks: Option<NetworksList>;
  updateNetworks: (list: NetworksList) => void;
};

const valueContext = {
  networks: {},
  updateNetworks: () => {},
};

const NetworksContext = React.createContext<NetworksContext>(valueContext);

export function useNetworks() {
  return useContext(NetworksContext);
}

function NetworksProvider({ children }: { children: React.ReactNode }) {
  const [networks, setNetworks] = useState<Option<NetworksList>>(undefined);

  useEffect(() => {
    let networksTmp = {};
    const response = localStorage.getItem('CHAIN_PARAMS');
    if (response) {
      const networksData: NetworksList = JSON.parse(response);
      networksTmp = { ...networksData };
    } else {
      networksTmp = { ...defaultNetworks };
      localStorage.setItem('CHAIN_PARAMS', JSON.stringify(defaultNetworks));
    }

    setNetworks((item) => ({
      ...item,
      ...networksTmp,
    }));
  }, []);

  const updateNetworks = (newList: NetworksList) => {
    localStorage.setItem('CHAIN_PARAMS', JSON.stringify(newList));
    setNetworks((item) => ({
      ...item,
      ...newList,
    }));
  };

  const value = useMemo(() => ({ networks, updateNetworks }), [networks, updateNetworks]);

  return <NetworksContext.Provider value={value}>{children}</NetworksContext.Provider>;
}

export default NetworksProvider;
