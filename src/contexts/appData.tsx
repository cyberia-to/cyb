import React, { useContext, useEffect, useMemo, useRef, useState } from 'react';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import useGetMarketData from 'src/hooks/useGetMarketData';
import useConvertMarketData from 'src/hooks/warp/useConvertMarketData';
import { ObjKeyValue } from 'src/types/data';
import { useWebsockets } from 'src/websockets/context';

type OptionObj<T> = T | object;

type ObjData = OptionObj<ObjKeyValue>;

type DataProviderContextType = {
  marketData: ObjData;
  dataTotalSupply: ObjData;
  block: number | null;
  filterParticles: string[];
};

const valueContext = {
  marketData: {},
  dataTotalSupply: {},
  block: null,
  filterParticles: [],
};

export const FILTERING_CONTRACT =
  'bostrom1p8drdvmwygrreesp4e425q6xs77zkcsj7z7h9as7sketuv5w334slsxv7l';

const DataProviderContext = React.createContext<DataProviderContextType>(valueContext);

export function useAppData() {
  return useContext(DataProviderContext);
}

function DataProvider({ children }: { children: React.ReactNode }) {
  const { marketData, dataTotal } = useGetMarketData();
  const convertMarketData = useConvertMarketData(marketData);
  const { cyber } = useWebsockets();
  const [blockHeight, setBlockHeight] = useState<number | null>(null);

  const filterContractQuery = useQueryContract(FILTERING_CONTRACT, {
    particles: {},
  });

  const filterParticlesRaw = filterContractQuery?.data?.map((item) => item[1]);
  const filterParticlesRef = useRef<string[]>([]);
  if (filterParticlesRaw && JSON.stringify(filterParticlesRaw) !== JSON.stringify(filterParticlesRef.current)) {
    filterParticlesRef.current = filterParticlesRaw;
  }
  const filterParticles = filterParticlesRef.current;

  const resultMarketData = Object.keys(convertMarketData).length ? convertMarketData : marketData;

  useEffect(() => {
    if (!cyber?.connected) {
      return;
    }

    const param = "tm.event='NewBlockHeader'";

    if (cyber.subscriptions.includes(param)) {
      return;
    }

    cyber.sendMessage({
      method: 'subscribe',
      params: [param],
      id: '1',
      jsonrpc: '2.0',
    });
  }, [cyber, cyber?.connected]);

  useEffect(() => {
    if (!cyber?.message?.result) {
      return;
    }

    const message = cyber?.message;

    if (
      Object.keys(message.result).length > 0 &&
      message.result.data.type === 'tendermint/event/NewBlockHeader'
    ) {
      const { height } = message.result.data.value.header;
      setBlockHeight(height);
    }
  }, [cyber?.message]);

  const valueMemo = useMemo(
    () => ({
      marketData: resultMarketData,
      dataTotalSupply: dataTotal,
      block: blockHeight,
      filterParticles,
    }),
    [resultMarketData, dataTotal, blockHeight, filterParticles]
  );

  return <DataProviderContext.Provider value={valueMemo}>{children}</DataProviderContext.Provider>;
}

export default DataProvider;
