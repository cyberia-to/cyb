import React, { useCallback, useContext, useMemo } from 'react';
import useAllDenomTraces from 'src/hooks/useAllDenomTraces';
import usePoolListInterval from 'src/hooks/usePoolListInterval';
import { Option } from 'src/types';
import { IbcDenomsArr, TracesDenomFuncResponse, TracesDenomFuncType } from 'src/types/ibc';
import { findPoolDenomInArr, isNative } from 'src/utils/utils';
import { useHub } from './hub';

type IbcDenomContextContextType = {
  ibcDenoms: Option<IbcDenomsArr>;
  tracesDenom: TracesDenomFuncType;
};

const valueContext = {
  ibcDenoms: undefined,
  tracesDenom: () => [],
};

const IbcDenomContext = React.createContext<IbcDenomContextContextType>(valueContext);

export function useIbcDenom() {
  return useContext(IbcDenomContext);
}

function IbcDenomProvider({ children }: { children: React.ReactNode }) {
  const poolsData = usePoolListInterval();
  const allDenomTraces = useAllDenomTraces();
  const { tokens } = useHub();

  const tracesDenom = useCallback<TracesDenomFuncType>(
    (denomTraces: string): TracesDenomFuncResponse[] => {
      const infoDenomTemp: TracesDenomFuncResponse = {
        denom: denomTraces,
        coinDecimals: 0,
        path: '',
        coinImageCid: '',
        native: true,
      };

      // pool token is array [denom1, denom2]
      if (denomTraces.includes('pool') && poolsData) {
        const findPool = findPoolDenomInArr(denomTraces, poolsData);
        if (findPool) {
          const { reserveCoinDenoms } = findPool;
          const denomA = tracesDenom(reserveCoinDenoms[0]);
          const denomB = tracesDenom(reserveCoinDenoms[1]);
          return [...denomA, ...denomB];
        }
      }

      // check hub contracts
      if (tokens?.[denomTraces]) {
        const { decimals, ticker, logo, channel_id: channelId, contract } = tokens[denomTraces];

        infoDenomTemp.denom = ticker;
        infoDenomTemp.coinDecimals = parseFloat(decimals);
        infoDenomTemp.coinImageCid = contract.includes('ibc') ? logo : '';
        infoDenomTemp.path = channelId;
      } else if (
        // response lcd all denom traces
        !isNative(denomTraces) &&
        allDenomTraces &&
        allDenomTraces[denomTraces]
      ) {
        const { baseDenom, sourceChannelId: sourceChannelIFromPath } = allDenomTraces[denomTraces];
        infoDenomTemp.native = false;
        infoDenomTemp.denom = baseDenom;
        infoDenomTemp.path = sourceChannelIFromPath;
      } else {
        infoDenomTemp.denom = denomTraces.toUpperCase();
      }

      return [{ ...infoDenomTemp }];
    },

    [allDenomTraces, poolsData, tokens]
  );

  const value = useMemo(
    () => ({ ibcDenoms: allDenomTraces, tracesDenom }),
    [allDenomTraces, tracesDenom]
  );

  return <IbcDenomContext.Provider value={value}>{children}</IbcDenomContext.Provider>;
}

export default IbcDenomProvider;
