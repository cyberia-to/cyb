import React, { useMemo, useState } from 'react';
import { matchPath, useLocation, useParams } from 'react-router-dom';
import { isPussyAddress } from 'src/utils/address';
import { CYBERVER_CONTRACTS, CYBERVER_CONTRACTS_LEGACY } from '../constants';
import { ContractTypes, ContractWithData, Economy, Metadata, SubnetInfo } from '../types';
import { cybernetRoutes } from './routes';
import useQueryCybernetContract from './useQueryCybernetContract.refactor';

const CybernetContext = React.createContext<{
  contracts: ContractWithData[];
  selectContract: (address: string) => void;
  selectedContract: ContractWithData;
  // subnetsQuery: ReturnType<typeof useQueryCybernetContract<SubnetInfo[]>;
  // fix
  subnetsQuery: {
    data: SubnetInfo[];
  };
  subnetsQuery: any;
}>({
  contracts: {},
  selectContract: null,
  subnetsQuery: null,
  selectedContract: null,
});

export function useCybernet() {
  return React.useContext(CybernetContext);
}

function useCybernetContractWithData(address: string) {
  const location = useLocation();

  const isMainPage = !!matchPath(cybernetRoutes.verse.path, location.pathname);

  const metadataQuery = useQueryCybernetContract<Metadata>({
    contractAddress: address,
    query: {
      get_verse_metadata: {},
    },
  });

  const economyQuery = useQueryCybernetContract<Economy>({
    contractAddress: address,
    query: {
      get_economy: {},
    },

    refetchInterval: isMainPage ? 20 * 1000 : undefined,
  });

  const { name, types } = metadataQuery.data || {};
  const type =
    name?.includes(ContractTypes.Graph) || types?.includes(ContractTypes.Graph)
      ? ContractTypes.Graph
      : ContractTypes.ML;

  return {
    address,
    type,
    metadata: {
      ...metadataQuery.data,
      name: CYBERVER_CONTRACTS_LEGACY.includes(address) ? '' : name,
    },
    economy: economyQuery.data,
    isLoading: metadataQuery.loading || economyQuery.loading,
  };
}

function CybernetProvider({ children }: { children: React.ReactNode }) {
  const [selectedContractAddress, setSelectedContractAddress] = useState(CYBERVER_CONTRACTS[0]);

  const { nameOrAddress } = useParams();

  const c1 = useCybernetContractWithData(CYBERVER_CONTRACTS[0]);
  const c2 = useCybernetContractWithData(CYBERVER_CONTRACTS[1]);

  const c3 = useCybernetContractWithData(CYBERVER_CONTRACTS_LEGACY[0]);
  const c4 = useCybernetContractWithData(CYBERVER_CONTRACTS_LEGACY[1]);

  const contracts = useMemo(() => [c1, c2, c3, c4], [c1, c2, c3, c4]);

  const currentContract =
    nameOrAddress &&
    contracts.find(
      (contract) => contract.address === nameOrAddress || contract.metadata?.name === nameOrAddress
    );

  let address;

  if (nameOrAddress && isPussyAddress(nameOrAddress)) {
    address = nameOrAddress;
  } else if (nameOrAddress && currentContract && currentContract.metadata?.name === nameOrAddress) {
    address = currentContract.address;
  }

  if (address && selectedContractAddress !== address) {
    setSelectedContractAddress(address);
  }

  const currentContract2 = contracts.find(
    (contract) => contract.address === selectedContractAddress
  );

  const subnetsQuery = useQueryCybernetContract<SubnetInfo[]>({
    query: {
      get_subnets_info: {},
    },
    contractAddress: address,
  });

  return (
    <CybernetContext.Provider
      value={useMemo(() => {
        console.log('verses', contracts);

        return {
          contracts,
          subnetsQuery,
          selectedContract: currentContract2,
        };
      }, [contracts, subnetsQuery, currentContract2])}
    >
      {children}
    </CybernetContext.Provider>
  );
}

export function useCurrentContract() {
  const { selectedContract } = useCybernet();

  const { metadata } = selectedContract;
  const contractName = metadata?.name;
  const type = selectedContract?.type;

  return {
    contractName: contractName || selectedContract.address,
    network: 'pussy',
    type,
  };
}

export default CybernetProvider;
