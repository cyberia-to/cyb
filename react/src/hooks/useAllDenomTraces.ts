import { useQuery } from '@tanstack/react-query';
import { QueryDenomTracesResponse } from 'cosmjs-types/ibc/applications/transfer/v1/query';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { setDenomTraces } from 'src/redux/features/ibcDenom';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import { IbcDenomsArr } from 'src/types/ibc';
import { getDenomHash } from 'src/utils/utils';

const keyQuery = 'allDenomTraces';

const mapDenomTraces = (denomTraces: QueryDenomTracesResponse['denomTraces']): IbcDenomsArr =>
  denomTraces.reduce((acc, value) => {
    const { path, baseDenom } = value;
    const ibcDenomHash = getDenomHash(path, baseDenom);

    const parts = path.split('/');
    const removetr = parts.filter((itemStr) => itemStr !== 'transfer');
    const sourceChannelId = removetr.join('/');

    return {
      ...acc,
      [ibcDenomHash]: {
        sourceChannelId,
        baseDenom,
        ibcDenom: ibcDenomHash,
      },
    };
  }, {});

function useAllDenomTraces() {
  const queryClient = useQueryClient();
  const dispatch = useAppDispatch();
  const { denomTraces: denomTracesLS } = useAppSelector((state) => state.ibcDenom);

  const [dataDenom, setDataDenom] = useState<Option<IbcDenomsArr>>(denomTracesLS);

  const { data } = useQuery(
    [keyQuery],
    () => {
      return queryClient?.allDenomTraces() as Option<QueryDenomTracesResponse>;
    },
    {
      enabled: Boolean(queryClient),
    }
  );

  useEffect(() => {
    if (!data) {
      return;
    }

    const ibcData = mapDenomTraces(data.denomTraces);

    if (Object.keys(ibcData).length > 0) {
      setDataDenom(ibcData);
      dispatch(setDenomTraces(ibcData));
    }
  }, [data, dispatch]);

  return dataDenom;
}

export default useAllDenomTraces;
