import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { useQueryClient } from 'src/contexts/queryClient';
import { makeTags } from '../../../../utils/utils';
import InstantiationContract from '../../contract/InstantiationContract';
import { FlexWrapCantainer } from '../../ui/ui';
import CodeInfo from './CodeInfo';
import styles from './styles.scss';
import TableInstance from './TableInstance';

const initDetails = {
  checksum: '',
  creator: '',
  data: '',
  id: '',
};

const useGetContractsInfo = (codeId, _updateFnc) => {
  const queryClient = useQueryClient();
  const [details, setDetails] = useState(initDetails);
  const [contracts, setContracts] = useState([]);
  const [uploadTxHash, setUploadTxHash] = useState('');

  useEffect(() => {
    const getContracts = async () => {
      if (queryClient) {
        const response = await queryClient.getContracts(codeId);
        console.log(`response getContracts`, response);
        setContracts(response);
      }
    };
    getContracts();
  }, [queryClient, codeId]);

  useEffect(() => {
    const getCodeDetails = async () => {
      if (queryClient) {
        const response = await queryClient.getCodeDetails(codeId);
        console.log(`response getCodeDetails`, response);
        setDetails(response);
      }
    };
    getCodeDetails();
  }, [queryClient, codeId]);

  useEffect(() => {
    const searchTx = async () => {
      if (queryClient) {
        const response = await queryClient.searchTx({
          tags: makeTags(
            `message.module=wasm&message.action=/cosmwasm.wasm.v1.MsgStoreCode&store_code.code_id=${codeId}`
          ),
        });
        const first = response.find(() => true);

        if (first.hash) {
          setUploadTxHash(first.hash);
        }
      }
    };
    searchTx();
  }, [queryClient, codeId]);

  return { uploadTxHash, contracts, details };
};

function CodePage() {
  const { codeId } = useParams();
  const [updateFnc, setUpdateFnc] = useState(0);
  const { uploadTxHash, contracts, details } = useGetContractsInfo(codeId, updateFnc);

  return (
    <MainContainer>
      <FlexWrapCantainer style={{ flexDirection: 'column', width: '60%', boxShadow: 'none' }}>
        <div className={styles.containerCodeDetailsHeader}>
          <div className={styles.containerCodeDetailsHeaderTitle}>Lib #{codeId}</div>
          <div className={styles.containerCodeDetailsHeaderFileInfo}>
            <div className={styles.containerCodeDetailsHeaderFileInfoType}>type: Wasm</div>
            <div>Size: {details.data.length > 0 && Math.round(details.data.length / 1024)} KiB</div>
          </div>
        </div>
        <CodeInfo uploadTxHash={uploadTxHash} details={details} />
        <InstantiationContract updateFnc={() => setUpdateFnc((item) => item + 1)} codeId={codeId} />
      </FlexWrapCantainer>

      <TableInstance contracts={contracts} />
    </MainContainer>
  );
}

export default CodePage;
