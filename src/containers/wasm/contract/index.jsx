import { Registry } from '@cosmjs/proto-signing';
import { Tx } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { MsgExecuteContract } from 'cosmjs-types/cosmwasm/wasm/v1/tx';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { BASE_DENOM } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { DenomArr, MainContainer } from '../../../components';
import { formatNumber, makeTags, trimString } from '../../../utils/utils';
import { CardCantainer, FlexWrapCantainer } from '../ui/ui';
import ExecuteContract from './ExecuteContract';
import ExecuteTable from './ExecuteTable';
import HistoryInfo from './HistoryInfo';
import InitializationInfo from './InitializationInfo';
import QueryContract from './QueryContract';
import RenderAbi from './renderAbi';
import styles from './stylesContractPage.scss';

function isStargateMsgExecuteContract(msg) {
  return msg.typeUrl === '/cosmwasm.wasm.v1.MsgExecuteContract' && !!msg.value;
}

const getAndSetDetails = async (client, contractAddress, setDetails) => {
  try {
    const response = await client.getContract(contractAddress);
    console.log(`response getAndSetDetails`, response);
    setDetails(response);
  } catch (error) {
    console.log(`error`, error);
  }
};

const getAndSetContractCodeHistory = async (client, contractAddress, setContractCodeHistory) => {
  try {
    const response = await client.getContractCodeHistory(contractAddress);
    console.log(`response getContractCodeHistory`, response);
    setContractCodeHistory(response);
  } catch (error) {
    console.log(`error`, error);
  }
};

const getAndSetInstantiationTxHash = async (client, contractAddress, setInstantiationTxHash) => {
  try {
    const response = await client.searchTx({
      tags: makeTags(
        `message.module=wasm&message.action=/cosmwasm.wasm.v1.MsgInstantiateContract&instantiate._contract_address=${contractAddress}`
      ),
    });
    const first = response.find(() => true);
    if (first.hash) {
      console.log(`first.hash`, first.hash);
      setInstantiationTxHash(first.hash);
    }
  } catch (error) {
    console.log(`error`, error);
  }
};

const getExecutionFromStargateMsgExecuteContract = (typeRegistry, tx) => {
  return (msg, i) => {
    const decodedMsg = typeRegistry.decode({
      typeUrl: msg.typeUrl,
      value: msg.value,
    });
    return {
      key: `${tx.hash}_${i}`,
      height: tx.height,
      transactionId: tx.hash,
      msg: decodedMsg,
    };
  };
};

const getExecutions = async (client, contractAddress, setExecutions) => {
  const typeRegistry = new Registry([['/cosmwasm.wasm.v1.MsgExecuteContract', MsgExecuteContract]]);

  const response = await client.searchTx({
    tags: makeTags(
      `message.module=wasm&message.action=/cosmwasm.wasm.v1.MsgExecuteContract&execute._contract_address=${contractAddress}`
    ),
  });

  const out = response.reduce((executions, tx) => {
    const decodedTx = Tx.decode(tx.tx);
    const txExecutions = decodedTx.body.messages
      .filter(isStargateMsgExecuteContract)
      .map(getExecutionFromStargateMsgExecuteContract(typeRegistry, tx));
    return [...executions, ...txExecutions];
  }, []);

  if (out.length > 0) {
    setExecutions(out.reverse());
  }
};

const getBalance = async (client, contractAddress, setBalance) => {
  try {
    const response = await client.getBalance(contractAddress, BASE_DENOM);
    if (response !== null) {
      setBalance(response);
    }
  } catch (error) {
    console.log(`error`, error);
    setBalance({ denom: '', amount: 0 });
  }
};

const useGetInfoContractAddress = (contractAddress, _updateFnc) => {
  const queryClient = useQueryClient();
  const [instantiationTxHash, setInstantiationTxHash] = useState('');
  const [contractCodeHistory, setContractCodeHistory] = useState([]);
  const [details, setDetails] = useState({});
  const [executions, setExecutions] = useState([]);
  const [balance, setBalance] = useState({ denom: '', amount: 0 });

  useEffect(() => {
    if (queryClient) {
      getAndSetDetails(queryClient, contractAddress, setDetails);
      getAndSetContractCodeHistory(queryClient, contractAddress, setContractCodeHistory);
      getAndSetInstantiationTxHash(queryClient, contractAddress, setInstantiationTxHash);
      getBalance(queryClient, contractAddress, setBalance);
    }
  }, [queryClient, contractAddress]);

  useEffect(() => {
    if (queryClient) {
      getExecutions(queryClient, contractAddress, setExecutions);
    }
  }, [queryClient, contractAddress]);

  return {
    balance,
    details,
    executions,
    contractCodeHistory,
    instantiationTxHash,
  };
};

function ContractPage() {
  const { contractAddress } = useParams();
  const [updateFnc, setUpdateFnc] = useState(0);
  const { balance, details, executions, instantiationTxHash, contractCodeHistory } =
    useGetInfoContractAddress(contractAddress, updateFnc);

  return (
    <MainContainer>
      <FlexWrapCantainer style={{ flexDirection: 'column', width: '60%', boxShadow: 'none' }}>
        <div className={styles.containerContractPageContainerTitle}>
          <div className={styles.containerContractPageContainerTitleTitle}>
            Contract: {trimString(contractAddress, 12)}
          </div>
          <div className={styles.containerContractPageContainerTitleBalance}>
            Balance: {formatNumber(parseFloat(balance.amount))}
            {balance.denom && (
              <>
                &nbsp;
                <DenomArr onlyImg marginContainer="0px 0px 0px 3px" denomValue={balance.denom} />
              </>
            )}
          </div>
        </div>

        <InitializationInfo initTxHash={instantiationTxHash} details={details} />
        <HistoryInfo contractCodeHistory={contractCodeHistory} />

        <QueryContract contractAddress={contractAddress} />

        <ExecuteContract contractAddress={contractAddress} />
      </FlexWrapCantainer>

      <FlexWrapCantainer style={{ width: '60%' }}>
        <RenderAbi
          contractAddress={contractAddress}
          updateFnc={() => setUpdateFnc((item) => item + 1)}
        />
      </FlexWrapCantainer>
      <CardCantainer style={{ width: '60%', margin: '0 auto' }}>
        <ExecuteTable executions={executions} />
      </CardCantainer>
    </MainContainer>
  );
}

export default ContractPage;
