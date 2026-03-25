import React, { useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { SubnetHyperParameters, SubnetInfo, SubnetNeuron } from 'src/features/cybernet/types';
import useCybernetContract from 'src/features/cybernet/ui/useQueryCybernetContract.refactor';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useCurrentSubnetGrades from './useCurrentSubnetGrades';

const SubnetContext = React.createContext<{
  subnetQuery: ReturnType<typeof useCybernetContract<SubnetInfo>>;
  hyperparamsQuery: ReturnType<typeof useCybernetContract<SubnetHyperParameters>>;
  neuronsQuery: ReturnType<typeof useCybernetContract<SubnetNeuron[]>>;

  addressRegisteredInSubnet: boolean;
  isRootSubnet: boolean;
  netuid: number;
  subnetRegistrationQuery: ReturnType<typeof useCybernetContract<number | null>>;
  grades: ReturnType<typeof useCurrentSubnetGrades>;

  // refetch: () => void;
}>({
  subnetQuery: null,
  neuronsQuery: null,
  hyperparamsQuery: null,
  addressRegisteredInSubnet: false,
  isRootSubnet: false,
  netuid: 0,
});

/**
 @deprecated
  */
export function useSubnet() {
  return React.useContext(SubnetContext);
}

export function useCurrentSubnet() {
  return useSubnet();
}

function SubnetProvider({ children }: { children: React.ReactNode }) {
  const { id } = useParams();
  const f = id === 'board' ? 0 : +id;
  const netuid = Number(f);
  const isRootSubnet = netuid === 0;

  const currentAddress = useCurrentAddress();

  const subnetQuery = useCybernetContract<SubnetInfo>({
    query: {
      get_subnet_info: {
        netuid,
      },
    },
  });

  const hyperparamsQuery = useCybernetContract<SubnetHyperParameters>({
    query: {
      get_subnet_hyperparams: {
        netuid,
      },
    },
  });

  const neuronsQuery = useCybernetContract<SubnetNeuron[]>({
    query: {
      get_neurons: {
        netuid,
      },
    },
  });

  const subnetRegistrationQuery = useCybernetContract<number | null>({
    query: {
      get_uid_for_hotkey_on_subnet: {
        netuid,
        hotkey: currentAddress,
      },
    },
  });

  const grades = useCurrentSubnetGrades({
    netuid,
    neuronsQuery,
    hyperparamsQuery,
  });

  const addressRegisteredInSubnet = subnetRegistrationQuery?.data !== null;

  const value = useMemo(() => {
    return {
      netuid,
      isRootSubnet,
      subnetQuery,
      hyperparamsQuery,
      neuronsQuery,
      addressRegisteredInSubnet,
      subnetRegistrationQuery,
      grades,
      // refetch: () => {
      //   subnetQuery.refetch();
      //   neuronsQuery.refetch();
      //   subnetUidQuery.refetch();
      // },
    };
  }, [
    addressRegisteredInSubnet,
    subnetQuery,
    neuronsQuery,
    grades,
    netuid,
    isRootSubnet,
    hyperparamsQuery,
    subnetRegistrationQuery,
    // subnetUidQuery,
  ]);

  return <SubnetContext.Provider value={value}>{children}</SubnetContext.Provider>;
}

export default SubnetProvider;
