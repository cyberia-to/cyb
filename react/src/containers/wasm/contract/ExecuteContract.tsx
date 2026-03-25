import { GasPrice } from '@cosmjs/launchpad';
import { useEffect, useState } from 'react';
import { Input } from 'src/components';
import Button from 'src/components/btnGrd';
import { BASE_DENOM } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { trimString } from 'src/utils/utils';
import { JsonView, LinkTx } from '../ui/ui';
import { JSONInputCard } from './InstantiationContract';
import styles from './stylesExecuteContract.scss';

const executePlaceholder = {
  transfer: {
    recipient: 'bostrom1p0r7uxstcw8ehrwuj4kn8qzzs0yypsjwxgd445',
    amount: '1',
  },
};

const coinsPlaceholder = [{ denom: BASE_DENOM, amount: '1' }];

const _gasPrice = GasPrice.fromString('0.001boot');

function ExecuteContract({ contractAddress }: { contractAddress: string }) {
  const { signer, signingClient } = useSigningClient();
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState(null);

  const [memo, setMemo] = useState('');

  const [msgObject, setMsgObject] = useState({});
  const [coinsObject, setCoinsObject] = useState({});

  const [executeResponse, setExecuteResponse] = useState({});

  useEffect(() => {
    setMsgObject({ result: executePlaceholder });
    setCoinsObject({ result: coinsPlaceholder });
  }, []);

  useEffect(() => {
    if (msgObject.error) {
      setError(msgObject.error);
      return;
    }

    if (executeResponse.error) {
      setError(executeResponse.error);
      return;
    }

    if (coinsObject.error) {
      setError(coinsObject.error);
      return;
    }

    setError(null);
  }, [coinsObject, executeResponse, msgObject]);

  const executeContract = async () => {
    if (!msgObject.result || !signer || !signingClient) {
      return;
    }

    setExecuting(true);

    try {
      const [{ address }] = await signer.getAccounts();

      const executeResponseResult = await signingClient.execute(
        address,
        contractAddress,
        msgObject.result,
        Soft3MessageFactory.fee(2),
        memo,
        coinsObject.result
      );

      console.log(`executeResponseResult`, executeResponseResult);
      setExecuteResponse({ result: executeResponseResult });
    } catch (e) {
      console.error(e);
      setExecuteResponse({ error: `Execute error: ${e}` });
    }

    setExecuting(false);
  };

  return (
    <div className={styles.containerExecuteContract}>
      <JSONInputCard
        placeholder={executePlaceholder}
        setState={setMsgObject}
        title="Execute contract"
        height="200px"
      />

      <JSONInputCard
        placeholder={coinsPlaceholder}
        setState={setCoinsObject}
        title="Coins to transfer"
        height="120px"
      />

      <div className={styles.containerExecuteContractInputContainer}>
        <div className={styles.containerExecuteContractInputContainerItem}>
          <span>Memo</span>
          <Input value={memo} onChange={(event) => setMemo(event.target.value)} />
        </div>
      </div>

      <Button
        pending={executing}
        pendingText="Executing"
        onClick={executeContract}
        disabled={!msgObject.result || !signer}
      >
        Execute contract
      </Button>

      {executeResponse.result && (
        <div className={styles.containerExecuteContractResult}>
          <span>Response:</span>

          <div>
            Txs:{' '}
            <LinkTx txs={executeResponse.result.transactionHash}>
              {trimString(executeResponse.result.transactionHash, 3, 3)}
            </LinkTx>
          </div>
          <JsonView src={executeResponse.result} />
        </div>
      )}

      {error && (
        <div className={styles.containerExecuteContractResult}>
          <div>{error}</div>
        </div>
      )}
    </div>
  );
}

export default ExecuteContract;
