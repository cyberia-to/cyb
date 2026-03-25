import { useEffect, useState } from 'react';
import Button from 'src/components/btnGrd';
import { useQueryClient } from 'src/contexts/queryClient';
import { JsonView } from '../ui/ui';
import { JSONInputCard } from './InstantiationContract';
import styles from './stylesQueryContract.scss';

const queryPlaceholder = {
  balance: { address: 'bostrom180tz4ahtyfhwnqwkpdqj3jelyxff4wlx2ymsv3' },
};

function QueryContract({ contractAddress }) {
  const queryClient = useQueryClient();
  const [error, setError] = useState(null);
  const [queryObject, setQueryObject] = useState({});
  const [queryResponse, setQueryResponse] = useState({});

  useEffect(() => {
    setQueryObject({ result: queryPlaceholder });
  }, []);

  useEffect(() => {
    if (queryObject.error) {
      setError(queryObject.error);
      return;
    }

    if (queryResponse.error) {
      setError(queryResponse.error);
      return;
    }

    setError(null);
  }, [queryObject, queryResponse]);

  const runQuery = async () => {
    if (!queryClient || !queryObject.result) {
      setError('Some error');
      return;
    }

    try {
      const queryResponseResult = await queryClient.queryContractSmart(
        contractAddress,
        queryObject.result
      );
      const formattedResult = JSON.stringify(queryResponseResult, null, '  ');
      setQueryResponse({ result: formattedResult });
    } catch (e) {
      setQueryResponse({ error: `Query error: ${e.message}` });
    }
  };

  return (
    <div className={styles.containerQueryContract}>
      <JSONInputCard
        placeholder={queryPlaceholder}
        setState={setQueryObject}
        title="Query contract"
        height="200px"
      />

      <Button onClick={runQuery} disabled={!queryClient || !queryObject?.result}>
        Run query
      </Button>

      {queryResponse.result && (
        <div className={styles.containerQueryContractMessage}>
          <div>Response:</div>
          <JsonView src={JSON.parse(queryResponse.result)} />
        </div>
      )}
      {error && (
        <div className={styles.containerQueryContractMessage}>
          <span className="text-danger" title="The contract query error">
            {error}
          </span>
        </div>
      )}
    </div>
  );
}

export default QueryContract;
