import { GasPrice } from '@cosmjs/launchpad';
import { useEffect, useState } from 'react';
import JSONInput from 'react-json-editor-ajrm';
import { Link } from 'react-router-dom';
import { Input } from 'src/components';
import Button from 'src/components/btnGrd';
import { BASE_DENOM } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { trimString } from '../../../utils/utils';
import { CardItem } from '../codes/code';
import { FlexWrapCantainer, jsonInputStyle } from '../ui/ui';
import RenderInstantiateMsg from './RenderInstantiateMsg';
import SelectFile from './renderAbi/SelectFile';
import useParseJsonSchema from './renderAbi/useParseJsonSchema';
import styles from './stylesInstantiationContract.scss';

const executePlaceholder = {
  name: 'Nation coin',
  symbol: 'NTN',
  decimals: 0,
  initial_balances: [
    {
      address: 'bostrom1p0r7uxstcw8ehrwuj4kn8qzzs0yypsjwxgd445',
      amount: '100000',
    },
  ],
  mint: {
    minter: 'bostrom1p0r7uxstcw8ehrwuj4kn8qzzs0yypsjwxgd445',
    cap: '1000000',
  },
};

const coinsPlaceholder = [{ denom: BASE_DENOM, amount: '1' }];
const _gasPrice = GasPrice.fromString('0.001boot');

export function JSONInputCard({ title, placeholder, setState, height }) {
  return (
    <div className={styles.containerJsonContractJSONInput}>
      <span className={styles.containerJsonContractJSONInputTitle}>{title}:</span>
      <JSONInput
        width="100%"
        height={height || '200px'}
        placeholder={placeholder}
        confirmGood={false}
        style={jsonInputStyle}
        onChange={({ jsObject }) => setState({ result: jsObject })}
      />
    </div>
  );
}

function InstantiationContract({ codeId, updateFnc }) {
  const { signer, signingClient } = useSigningClient();
  const [error, setError] = useState(null);
  const [executing, setExecuting] = useState(false);
  const [executeResponse, setExecuteResponse] = useState({});

  const [memo, setMemo] = useState('');
  const [label, setLabel] = useState('');

  const [msgObject, setMsgObject] = useState({});
  const [coinsObject, setCoinsObject] = useState({});

  const [fileAbiExecute, setFileAbiExecute] = useState(null);
  const { dataObj: schemaExecute } = useParseJsonSchema(fileAbiExecute);

  let content;

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

    setError(undefined);
  }, [coinsObject, executeResponse, msgObject]);

  const executeContract = async () => {
    if (!msgObject.result || !label || !signer || !signingClient) {
      setError('Some error');
      return;
    }

    setExecuting(true);

    try {
      const [{ address }] = await signer.getAccounts();

      const executeResponseResult = await signingClient.instantiate(
        address,
        parseFloat(codeId),
        msgObject.result,
        label,
        Soft3MessageFactory.fee(3),
        {
          memo,
          funds: coinsObject.result,
        }
      );
      console.log(`executeResponseResult`, executeResponseResult);
      setExecuteResponse({ result: executeResponseResult });
    } catch (e) {
      setExecuteResponse({ error: `Execute error: ${e}` });
    }

    setExecuting(false);
  };

  const updateFncInst = () => {
    if (updateFnc) {
      updateFnc();
    }
    setFileAbiExecute(null);
    setLabel('');
    setMemo('');
  };

  if (fileAbiExecute === null) {
    if (label.length) {
      content = (
        <div>
          <SelectFile text="Upload instantiate schema" stateCallback={setFileAbiExecute} />
        </div>
      );
    }
  } else {
    content = (
      <RenderInstantiateMsg
        schema={schemaExecute}
        codeId={codeId}
        memo={memo}
        label={label}
        updateFnc={updateFncInst}
      />
    );
  }

  return (
    <div className={styles.containerJsonContract}>
      <JSONInputCard
        placeholder={executePlaceholder}
        setState={setMsgObject}
        title="Instantiate contract"
        height="200px"
      />

      <JSONInputCard
        placeholder={coinsPlaceholder}
        setState={setCoinsObject}
        title="Coins to transfer"
        height="120px"
      />
      <div className={styles.containerJsonContractInputContainer}>
        <div className={styles.containerJsonContractInputContainerItem}>
          <span>Label *</span>
          <Input value={label} onChange={(event) => setLabel(event.target.value)} />
        </div>
        <div className={styles.containerJsonContractInputContainerItem}>
          <span>Memo</span>
          <Input value={memo} onChange={(event) => setMemo(event.target.value)} />
        </div>
      </div>

      <Button
        pending={executing}
        pendingText="Executing"
        onClick={executeContract}
        disabled={!msgObject.result || !signer || !label}
      >
        Instantiate contract
      </Button>

      {executeResponse.result && (
        <div className={styles.containerJsonContractResult}>
          <span>Response:</span>
          <CardItem
            title="Contract"
            value={
              <Link to={`/contracts/${executeResponse.result.contractAddress}`}>
                {trimString(executeResponse.result.contractAddress, 10)}
              </Link>
            }
          />
          <CardItem
            title="Tx"
            value={
              <Link to={`/network/bostrom/tx/${executeResponse.result.transactionHash}`}>
                {trimString(executeResponse.result.transactionHash, 8, 8)}
              </Link>
            }
          />
        </div>
      )}

      {content && (
        <FlexWrapCantainer style={{ flexDirection: 'column' }}>{content}</FlexWrapCantainer>
      )}

      {error && (
        <div>
          <div>{error}</div>
        </div>
      )}
    </div>
  );
}

export default InstantiationContract;
