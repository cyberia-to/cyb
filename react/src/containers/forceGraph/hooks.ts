import { useEffect, useState } from 'react';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useCyberlinksByParticleQuery } from 'src/generated/graphql';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';
import { QUERY_GET_FOLLOWERS } from './query';

const useGetDataGql = () => {
  const { fetchWithDetails } = useQueueIpfsContent();
  const { data: dataGql, loading: loadingGql } = useCyberlinksByParticleQuery({
    variables: {
      limit: 1000,
      where: QUERY_GET_FOLLOWERS,
      offset: 0,
    },
  });
  const [data, setData] = useState([]);

  useEffect(() => {
    if (!loadingGql && fetchWithDetails) {
      if (dataGql) {
        const { cyberlinks } = dataGql;

        cyberlinks.forEach(async (item) => {
          const addressResolve = fetchWithDetails
            ? (await fetchWithDetails(item.to)).content
            : null;
          if (addressResolve?.match(PATTERN_CYBER)) {
            setData((itemData) => [
              ...itemData,
              {
                to: addressResolve,
                subject: item.neuron,
                txhash: item.transaction_hash,
              },
            ]);
          }
        });
      }
    }
  }, [dataGql, loadingGql, fetchWithDetails]);

  return { data };
};

export default useGetDataGql;
