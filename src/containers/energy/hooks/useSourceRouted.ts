import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

// TODO: move to soft.js
export type DestinationRoute = {
  source: string;
  destination: string;
  name: string;
  value: any[];
  alias: string;
  resource: {
    milliampere?: number;
    millivolt?: number;
  };
};

const initValue = {
  millivolt: 0,
  milliampere: 0,
};

const getCalculationBalance = (data) => {
  const balances = {};
  if (Object.keys(data).length > 0) {
    data.forEach((item) => {
      balances[item.denom] = parseFloat(item.amount);
    });
  }

  return balances;
};

// refactor this...
function useGetSourceRoutes(addressActive: string) {
  const queryClient = useQueryClient();
  const [sourceRouted, setSourceRouted] = useState([]);
  const [destinationRoutes, setDestinationRoutes] = useState<DestinationRoute[] | []>([]);
  const [sourceEnergy, setSourceEnergy] = useState(initValue);
  const [destinationEnergy, setDestinationEnergy] = useState(initValue);

  // temp to reduce refactoring
  const [update, setUpdate] = useState(0);

  useEffect(() => {
    const sourceRoutedGet = async () => {
      setSourceRouted([]);

      if (queryClient && addressActive !== null) {
        try {
          const queryResultsourceRoutes = await queryClient.sourceRoutes(addressActive);
          if (queryResultsourceRoutes.routes) {
            queryResultsourceRoutes.routes.forEach((item, index) => {
              queryResultsourceRoutes.routes[index].resource = {
                milliampere: 0,
                millivolt: 0,
              };
              const value = getCalculationBalance(item.value);
              queryResultsourceRoutes.routes[index].resource = {
                ...queryResultsourceRoutes.routes[index].resource,
                ...value,
              };
            });
            setSourceRouted(queryResultsourceRoutes.routes);
          }
        } catch (error) {
          console.log(error);
          setSourceRouted([]);
        }
      }
    };
    sourceRoutedGet();
  }, [queryClient, addressActive]);

  useEffect(() => {
    const sourceRoutedGet = async () => {
      setSourceEnergy(initValue);

      if (queryClient && addressActive !== null) {
        try {
          const queryResultsourceRoutedEnergy = await queryClient.sourceRoutedEnergy(addressActive);
          if (queryResultsourceRoutedEnergy.value) {
            const { value } = queryResultsourceRoutedEnergy;
            const sourceRoutedEnergy = getCalculationBalance(value);
            if (sourceRoutedEnergy.milliampere) {
              setSourceEnergy((item) => ({
                ...item,
                milliampere: sourceRoutedEnergy.milliampere,
              }));
            }
            if (sourceRoutedEnergy.millivolt) {
              setSourceEnergy((item) => ({
                ...item,
                millivolt: sourceRoutedEnergy.millivolt,
              }));
            }
          }
        } catch (error) {
          console.log(error);
          setSourceEnergy(initValue);
        }
      }
    };
    sourceRoutedGet();
  }, [queryClient, addressActive]);

  useEffect(() => {
    setDestinationRoutes([]);
    const sourceRoutedGet = async () => {
      if (queryClient && addressActive !== null) {
        try {
          const queryResultdestinationRoutes = (await queryClient.destinationRoutes(
            addressActive
          )) as {
            routes: DestinationRoute[] | undefined;
          };
          if (queryResultdestinationRoutes.routes) {
            queryResultdestinationRoutes.routes.forEach((item, index) => {
              queryResultdestinationRoutes.routes[index].resource = {
                milliampere: 0,
                millivolt: 0,
              };
              const value = getCalculationBalance(item.value);
              queryResultdestinationRoutes.routes[index].resource = {
                ...queryResultdestinationRoutes.routes[index].resource,
                ...value,
              };
            });
            setDestinationRoutes(queryResultdestinationRoutes.routes);
          }
        } catch (error) {
          console.log(error);
          setDestinationRoutes([]);
        }
      }
    };
    sourceRoutedGet();
  }, [queryClient, addressActive]);

  useEffect(() => {
    const sourceRoutedGet = async () => {
      setDestinationEnergy(initValue);
      if (queryClient && addressActive !== null) {
        try {
          const queryResultdestinationRoutedEnergy =
            await queryClient.destinationRoutedEnergy(addressActive);
          if (queryResultdestinationRoutedEnergy.value) {
            const { value } = queryResultdestinationRoutedEnergy;
            const destinationRoutedEnergy = getCalculationBalance(value);
            if (destinationRoutedEnergy.milliampere) {
              setDestinationEnergy((item) => ({
                ...item,
                milliampere: destinationRoutedEnergy.milliampere,
              }));
            }
            if (destinationRoutedEnergy.millivolt) {
              setDestinationEnergy((item) => ({
                ...item,
                millivolt: destinationRoutedEnergy.millivolt,
              }));
            }
          }
        } catch (error) {
          console.log(error);
          setDestinationEnergy(initValue);
        }
      }
    };
    sourceRoutedGet();
  }, [queryClient, addressActive]);

  return {
    sourceRouted,
    sourceEnergy,
    destinationRoutes,
    destinationEnergy,
    update: () => setUpdate(update + 1),
  };
}

export default useGetSourceRoutes;
