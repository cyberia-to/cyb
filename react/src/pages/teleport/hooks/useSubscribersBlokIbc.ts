import { useEffect, useState } from 'react';
import { Option } from 'src/types';
import TxTracer from '../../../features/ibc-history/tx/TracerTx';

const wsEndpoint = '/websocket';

type ChainHeight = {
  height: string;
  chainId: string;
};

function useSubscribersBlokIbc(client) {
  const [blockInfo, setBlockInfo] = useState<ChainHeight | undefined>(undefined);

  useEffect(() => {
    const url = client?.tmClient?.client?.url;
    console.debug('url', url);
    let traceTx: Option<TxTracer>;

    if (url) {
      traceTx = new TxTracer(url, wsEndpoint);

      traceTx.subscribeBlock((result) => {
        if (result?.block?.header) {
          const { height, chain_id: chainId } = result.block.header;
          setBlockInfo({ height, chainId });
        }
      });
    }

    return () => {
      if (traceTx) {
        traceTx.close();
      }
    };
  }, [client]);

  return { blockInfo };
}

export default useSubscribersBlokIbc;
