import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { useDevice } from 'src/contexts/device';
import { useAdviser } from 'src/features/adviser/context';
import ActionBarContainer from '../Search/ActionBarContainer';
import { getTxs } from './api/data';
import { mapResponseDataGetTxs } from './api/mapping';
import InformationTxs from './informationTxs';
import Msgs from './msgs';
import { ValueInformation } from './type';

function TxsDetails() {
  const { isMobile: mobile } = useDevice();
  const { txHash } = useParams();
  const [msgs, setMsgs] = useState();
  const [information, setInformation] = useState<ValueInformation>();
  const { setAdviser } = useAdviser();

  useEffect(() => {
    getTxs(txHash || '').then((response) => {
      if (!response) {
        return;
      }

      const { info, messages, rawLog } = mapResponseDataGetTxs(response);
      setInformation({ ...info });
      setMsgs(messages);

      if (rawLog) {
        setAdviser(rawLog, 'red');
      }
    });
  }, [txHash, setAdviser]);

  return (
    <>
      <MainContainer>
        <InformationTxs data={information} />
        {msgs && <Msgs data={msgs} />}
      </MainContainer>
      {!mobile && <ActionBarContainer valueSearchInput={txHash} keywordHash={txHash} />}
    </>
  );
}

export default TxsDetails;
