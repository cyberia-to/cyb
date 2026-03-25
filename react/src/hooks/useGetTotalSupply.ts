import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';
import { useHub } from 'src/contexts/hub';
import { useQueryClient } from 'src/contexts/queryClient';
import { Option } from 'src/types';
import { ObjKeyValue } from 'src/types/data';
import { isNative, reduceBalances } from 'src/utils/utils';

type OptionInterval = {
  refetchInterval?: number | false;
};

const defaultTokenList = {
  [BASE_DENOM]: 0,
  [DENOM_LIQUID]: 0,
  milliampere: 0,
  millivolt: 0,
  tocyb: 0,
};

const totalSupplyFetcher = (client: Option<CyberClient>) => {
  if (!client) {
    return undefined;
  }

  return client.totalSupply();
};

function useGetTotalSupply(option: OptionInterval = { refetchInterval: false }) {
  const queryClient = useQueryClient();
  const { tokens } = useHub();
  const [totalSupplyAll, setTotalSupplyAll] = useState<Option<ObjKeyValue>>(undefined);
  const [totalSupplyProofList, setTotalSupplyProofList] = useState<Option<ObjKeyValue>>(undefined);
  const { data: dataGetTotalSupply } = useQuery(
    ['totalSupply'],
    () => totalSupplyFetcher(queryClient),
    {
      enabled: Boolean(queryClient),
      refetchInterval: option.refetchInterval,
    }
  );

  useEffect(() => {
    const getTotalSupply = async () => {
      if (dataGetTotalSupply) {
        const datareduceTotalSupply = reduceBalances(dataGetTotalSupply);

        if (Object.keys(datareduceTotalSupply).length > 0) {
          setTotalSupplyAll(datareduceTotalSupply);
        }

        const reduceData = {};

        if (tokens) {
          Object.keys(datareduceTotalSupply).forEach((key) => {
            const value = datareduceTotalSupply[key];
            if (!isNative(key)) {
              if (tokens[key]) {
                reduceData[key] = value;
              }
            } else if (key.indexOf('pool') !== 0) {
              reduceData[key] = value;
            }
          });
        }

        if (Object.keys(reduceData).length > 0) {
          setTotalSupplyProofList({ ...defaultTokenList, ...reduceData });
        } else {
          setTotalSupplyProofList({
            ...defaultTokenList,
            ...datareduceTotalSupply,
          });
        }
      }
    };

    getTotalSupply();
  }, [tokens, dataGetTotalSupply]);

  return { totalSupplyProofList, totalSupplyAll };
}

export default useGetTotalSupply;
