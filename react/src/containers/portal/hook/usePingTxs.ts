import { useEffect, useState } from 'react';
import portalConfirmed from 'sounds/portalConfirmed112.mp3';
import { useQueryClient } from 'src/contexts/queryClient';

const portalConfirmedObg = new Audio(portalConfirmed);

const playPortalConfirmed = () => {
  portalConfirmedObg.play();
};

export type TxHash = {
  txHash: string;
  status: 'pending' | 'confirmed' | 'error';
  rawLog?: string;
};

function usePingTxs() {
  const queryClient = useQueryClient();
  const [txHash, setTxHash] = useState<null | TxHash>(null);
  const [updateFunc, setUpdateFunc] = useState(0);

  useEffect(() => {
    if (txHash !== null && txHash.status !== 'pending') {
      setTimeout(() => setTxHash(null), 35000);
    }
  }, [txHash]);

  useEffect(() => {
    const confirmTx = async () => {
      if (queryClient && txHash && txHash.status === 'pending') {
        const response = await queryClient.getTx(txHash.txHash);
        console.log('response :>> ', response);
        if (response && response !== null) {
          if (response.code === 0) {
            setTxHash((item) => ({
              ...item,
              status: 'confirmed',
            }));
            setUpdateFunc((item) => item + 1);
            try {
              playPortalConfirmed();
            } catch (error) {
              console.log('error', error);
            }
            return;
          }
          if (response.code) {
            setTxHash((item) => ({
              ...item,
              status: 'error',
              rawLog: response.rawLog.toString(),
            }));
            // setErrorMessage(response.rawLog);
            return;
          }
        }
        setTimeout(confirmTx, 1500);
      }
    };
    confirmTx();
  }, [queryClient, txHash]);

  const updateTxHash = (data: TxHash) => {
    setTxHash(data);
  };

  return {
    updateTxHash,
    updateFunc,
    txHash,
  };
}

export default usePingTxs;
