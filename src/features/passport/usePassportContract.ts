import { equals } from 'ramda';
import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { PassportContractQuery, queryPassportContract } from 'src/services/soft.js/api/passport';

type Props = {
  query: PassportContractQuery;
  skip?: boolean;
};

// TODO: use  passport redux hook, or useQuery, think
function usePassportContract<DataType>({ query, skip }: Props) {
  const [data, setData] = useState<DataType>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState();

  const queryClient = useQueryClient();

  const lastQuery = useRef<PassportContractQuery>();

  async function queryContract(query: PassportContractQuery) {
    if (!queryClient) {
      return;
    }

    try {
      setLoading(true);
      setData(undefined);
      setError(undefined);

      const response = await queryPassportContract(query, queryClient);
      setData(response);
    } catch (error) {
      console.error(error);
      setData(null);
      setError(error);
      setData(null);
    }
    setLoading(false);
  }

  useEffect(() => {
    if (skip) {
      return;
    }

    if (equals(lastQuery.current, query)) {
      return;
    }

    lastQuery.current = query;
    queryContract(query);
  }, [query, skip, queryContract]);

  return {
    data,
    loading,
    error,
  };
}

export default usePassportContract;
