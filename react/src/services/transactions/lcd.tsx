import {
  GetTxsEventRequest,
  GetTxsEventResponse,
  GetTxsEventResponseAmino,
  OrderBy,
} from '@cybercongress/cyber-ts/cosmos/tx/v1beta1/service';
import axios from 'axios';
import { CID_FOLLOW, CID_TWEET } from 'src/constants/app';
import { LCD_URL } from 'src/constants/config';
import { LinksType, LinksTypeFilter } from 'src/containers/Search/types';

type PropsTx = {
  events: ReadonlyArray<{ key: string; value: string }>;
  pagination?: GetTxsEventRequest['pagination'];
  orderBy?: GetTxsEventRequest['orderBy'];
};

export const getTxs = async (txsHash: string) => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/cosmos/tx/v1beta1/txs/${txsHash}`,
    });
    return response.data;
  } catch (e) {
    console.error(e);
    return null;
  }
};

/**
 * @deprecated don't use lcd, use cyber-ts instead
 */
export async function getTransactions({
  events,
  pagination = { limit: 20, offset: 0 },
  orderBy,
  config,
}: PropsTx) {
  try {
    const { offset, limit } = pagination;
    const response = await axios.get<GetTxsEventResponseAmino>(`${LCD_URL}/cosmos/tx/v1beta1/txs`, {
      params: {
        'pagination.offset': offset,
        'pagination.limit': limit,
        order_by: orderBy,
        events: events.map((evn) => `${evn.key}='${evn.value}'`),
      },
      paramsSerializer: {
        indexes: null,
      },
      signal: config?.signal,
    });

    const { txs } = response.data;

    // bullshit formatting FIXME:
    // const formatted = GetTxsEventResponse.fromAmino(response.data);
    // from amino to protobuf
    const formatted = {
      txs,
      pagination: response.data.pagination || {
        total: response.data.total,
      },
      txResponses: response.data.tx_responses,
    } as GetTxsEventResponse;

    if (!formatted.pagination?.total) {
      formatted.pagination.total = formatted.txResponses.length;
    }

    return formatted;
  } catch (e) {
    console.error('getTransactions failed:', e);
    return null;
  }
}

const getLink = async (
  cid: string,
  type: LinksType = LinksTypeFilter.from,
  { offset, limit, order = OrderBy.ORDER_BY_DESC }
) => {
  try {
    const response = await getTransactions({
      events: [
        {
          key: `cyberlink.particle${type === LinksTypeFilter.to ? 'To' : 'From'}`,
          value: cid,
        },
      ],
      pagination: {
        limit,
        offset,
      },
      orderBy: order,
    });

    return response;
  } catch (e) {
    console.log(e);
    return null;
  }
};

export const getFromLink = async (cid, offset, limit) => {
  return getLink(cid, LinksTypeFilter.from, { offset, limit });
};

export const getToLink = async (cid, offset, limit) => {
  return getLink(cid, LinksTypeFilter.to, { offset, limit });
};

export const getFollows = async (address) => {
  try {
    const response = await getTransactions({
      events: [
        { key: 'cyberlink.particleFrom', value: CID_FOLLOW },
        { key: 'cyberlink.neuron', value: address },
      ],
      pagination: { limit: 1000000000 },
    });

    return response;
  } catch (e) {
    console.log(e);
    return null;
  }
};

export const getTweet = async (address) => {
  try {
    const response = await getTransactions({
      events: [
        { key: 'cyberlink.particleFrom', value: CID_TWEET },
        { key: 'cyberlink.neuron', value: address },
      ],
      pagination: { limit: 1000000000 },
    });
    return response;
  } catch (error) {
    console.log(error);
    return null;
  }
};

export const checkFollow = async (address, addressFollowHash) => {
  try {
    const response = await getTransactions({
      events: [
        { key: 'cyberlink.particleFrom', value: CID_FOLLOW },
        { key: 'cyberlink.neuron', value: address },
        { key: 'cyberlink.particleTo', value: addressFollowHash },
      ],
      pagination: { limit: 1000000000 },
    });
    return response;
  } catch (error) {
    console.log(error);
    return null;
  }
};

export const getFollowers = async (addressHash) => {
  try {
    const response = await getTransactions({
      events: [
        {
          key: 'cyberlink.particleFrom',
          value: CID_FOLLOW,
        },
        {
          key: 'cyberlink.particleTo',
          value: addressHash,
        },
      ],
      pagination: {
        limit: 1000000000,
      },
    });

    return response;
  } catch (error) {
    console.log(error);
    return null;
  }
};
