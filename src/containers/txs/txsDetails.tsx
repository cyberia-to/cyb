import { useEffect, useState, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { useDevice } from 'src/contexts/device';
import { useAdviser } from 'src/features/adviser/context';
import ActionBar from 'src/components/actionBar';
import { friendlyErrorMessage } from 'src/utils/errorMessages';
import ActionBarContainer from '../Search/ActionBarContainer';
import { getTxs } from './api/data';
import { mapResponseDataGetTxs } from './api/mapping';
import InformationTxs from './informationTxs';
import Msgs from './msgs';
import { ValueInformation } from './type';

const POLL_INTERVAL = 5000;

function TxsDetails() {
  const { isMobile: mobile } = useDevice();
  const { txHash } = useParams();
  const navigate = useNavigate();
  const [msgs, setMsgs] = useState();
  const [information, setInformation] = useState<ValueInformation>();
  const { setAdviser } = useAdviser();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    const fetchTx = () => {
      getTxs(txHash || '').then((response) => {
        if (!response) {
          return;
        }

        const { info, messages, rawLog } = mapResponseDataGetTxs(response);
        setInformation({ ...info });
        setMsgs(messages);

        if (rawLog) {
          setAdviser(friendlyErrorMessage(rawLog), 'red');
        }

        // Stop polling once tx is confirmed (has height)
        if (info.height && timerRef.current) {
          clearInterval(timerRef.current);
          timerRef.current = null;
        }
      });
    };

    fetchTx();

    // Poll until tx is confirmed
    timerRef.current = setInterval(fetchTx, POLL_INTERVAL);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [txHash, setAdviser]);

  return (
    <>
      <MainContainer>
        <InformationTxs data={information} />
        {msgs && <Msgs data={msgs} />}
      </MainContainer>
      <ActionBar onClickBack={() => navigate(-1)} />
    </>
  );
}

export default TxsDetails;
