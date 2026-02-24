import { TxsResponse } from '@cosmjs/launchpad';
import { OrderBy } from '@cybercongress/cyber-ts/cosmos/tx/v1beta1/service';
import { InfiniteData, useInfiniteQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useState } from 'react';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { getTransactions } from 'src/services/transactions/lcd';
import { Nullable, Option } from 'src/types';
import { AccountValue } from 'src/types/defaultAccount';

const limit = 5;

// use hook from cyber-ts
const getSendBySenderRecipient = async (address, offset = 0, limit = 5) => {
  try {
    const { recipient, sender } = address;

    const response = await getTransactions({
      events: [
        { key: 'message.action', value: '/cosmos.bank.v1beta1.MsgSend' },
        { key: 'transfer.sender', value: sender },
        { key: 'transfer.recipient', value: recipient },
      ],
      pagination: { limit, offset },
      orderBy: OrderBy.ORDER_BY_DESC,
    });

    return response;
  } catch (e) {
    console.log(e);
    return undefined;
  }
};

const concatResponse = (arr: undefined | InfiniteData<{ data: any }>) => {
  return (
    arr?.pages?.reduce((acc, page) => {
      return acc.concat(page.data.txResponses);
    }, []) || []
  );
};

function useGetSendBySenderRecipient(
  addressSender: Option<string>,
  addressRecipient: Option<string>,
  callBack?: React.Dispatch<React.SetStateAction<number>>
) {
  const data = useInfiniteQuery(
    ['getSendBySenderRecipient', addressSender, addressRecipient],
    async ({ pageParam = 0 }) => {
      const address = {
        sender: addressSender,
        recipient: addressRecipient,
      };

      const offset = new BigNumber(limit).multipliedBy(pageParam).toNumber();

      const response = await getSendBySenderRecipient(address, offset, limit);

      if (callBack && offset === 0 && response) {
        callBack(response.pagination?.total);
      }

      return {
        data: response,
        page: pageParam,
      };
    },
    {
      enabled:
        Boolean(addressSender) &&
        Boolean(addressSender?.match(PATTERN_CYBER)) &&
        Boolean(addressRecipient) &&
        Boolean(addressRecipient?.match(PATTERN_CYBER)),
      getNextPageParam: (lastPage) => {
        const { page, data } = lastPage;

        const total = data?.pagination?.total || 0;

        if (!total || (page + 1) * limit >= total) {
          return undefined;
        }

        return page + 1;
      },
    }
  );

  return {
    data: data.data,
    fetchNextPage: data.fetchNextPage,
    refetch: data.refetch,
    hasNextPage: data.hasNextPage,
  };
}

function useGetSendTxsByAddressByLcd(
  sender: Nullable<AccountValue>,
  addressRecipient: Option<string>
) {
  const [addressSender, setAddressSender] = useState<Option<string>>();
  const [dataTsx, setDataTxs] = useState<Option<TxsResponse[]>>(undefined);

  const dataSend = useGetSendBySenderRecipient(addressSender, addressRecipient);
  const dataReceive = useGetSendBySenderRecipient(addressRecipient, addressSender);

  useEffect(() => {
    let firstSendItem = '0';
    let lastSendItem = '0';
    const dataSendArr: TxsResponse[] = concatResponse(dataSend.data);
    const dataReceiveArr: TxsResponse[] = concatResponse(dataReceive.data);

    let dataTxs: TxsResponse[] = [];

    if (dataSendArr.length) {
      lastSendItem = dataSendArr[dataSendArr.length - 1].height;
      firstSendItem = dataSendArr[0].height;
    }

    if (dataSendArr.length && dataReceiveArr.length) {
      dataReceiveArr.forEach((item) => {
        const height = parseFloat(item.height);
        if (height > parseFloat(firstSendItem)) {
          dataTxs.push(item);
        }

        if (height > parseFloat(lastSendItem) && height < parseFloat(firstSendItem)) {
          dataTxs.push(item);
        }

        if (height < parseFloat(lastSendItem)) {
          dataTxs.push(item);
        }
      });
      dataTxs = [...dataTxs, ...dataSendArr];
      dataTxs = dataTxs.sort((itemA, itemB) => parseFloat(itemB.height) - parseFloat(itemA.height));
    } else {
      dataTxs = [...dataSendArr, ...dataReceiveArr];
    }

    setDataTxs(dataTxs);
  }, [dataSend.data, dataReceive.data]);

  useEffect(() => {
    if (sender) {
      setAddressSender(sender.bech32);
    }
  }, [sender]);

  const fetchNextPage = useCallback(() => {
    dataReceive.fetchNextPage();
    dataSend.fetchNextPage();
  }, [dataReceive, dataSend]);

  const refetch = useCallback(() => {
    dataReceive.refetch();
    dataSend.refetch();
  }, [dataReceive, dataSend]);

  return {
    data: dataTsx,
    fetchNextPage,
    refetch,
    hasNextPage: Boolean(dataReceive.hasNextPage || dataSend.hasNextPage),
  };
}

export default useGetSendTxsByAddressByLcd;
