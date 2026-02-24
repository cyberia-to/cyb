import { useEffect, useState } from 'react';
import { ObjectKey } from 'src/types/data';
import { Network } from 'src/types/hub';
import { Networks } from 'src/types/networks';
import { useNetworks } from './useHub';

export function isNativeChainId(chain: string) {
  // temp
  if (chain && Networks[chain.toUpperCase()]) {
    return true;
  }
  // also temp
  if (chain && chain === 'space-pussy') {
    return true;
  }
  return false;
}

const findChainIdInNetworkList = (chainId: string, networks: ObjectKey<Network>) => {
  if (Object.hasOwn(networks, chainId)) {
    return networks[chainId];
  }

  return undefined;
};

export const useTracesNetworks = (chainIdTraces: string) => {
  const { networks } = useNetworks();
  const [chainInfo, setChainInfo] = useState({
    chainId: chainIdTraces,
    chainName: chainIdTraces,
    chainIdImageCid: '',
  });

  useEffect(() => {
    if (networks) {
      let infoTemp = {};
      const chainInfoFromList = findChainIdInNetworkList(chainIdTraces, networks);

      if (chainInfoFromList) {
        const { chain_id: chainId, name: chainName, logo: chainIdImageCid } = chainInfoFromList;
        infoTemp = {
          chainId,
          chainName,
          chainIdImageCid,
        };
      }

      if (Object.keys(infoTemp).length > 0) {
        setChainInfo((item) => ({ ...item, ...infoTemp }));
      }
    } else {
      const infoTemp = {
        chainId: chainIdTraces,
        chainName: chainIdTraces,
        chainIdImageCid: '',
      };
      setChainInfo((item) => ({ ...item, ...infoTemp }));
    }
  }, [chainIdTraces, networks]);

  return { chainInfo };
};
