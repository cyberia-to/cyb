import {
  MessagesByAddressCountDocument,
  MessagesByAddressCountQuery,
  MessagesByAddressCountQueryVariables,
  MessagesByAddressSenseDocument,
  MessagesByAddressSenseQuery,
  MessagesByAddressSenseQueryVariables,
} from 'src/generated/graphql';
import { NeuronAddress } from 'src/types/base';
import { fetchIterableByOffset } from 'src/utils/async/iterable';
import { numberToUtcDate } from 'src/utils/date';
import { Transaction } from './types';
import { createIndexerClient } from './utils/graphqlClient';

type OrderDirection = 'desc' | 'asc';
type Abortable = { abortSignal: AbortSignal };

export type MessagesByAddressVariables = {
  neuron: NeuronAddress;
  timestampFrom: number;
  offset?: number;
  types: Transaction['type'][];
  orderDirection: OrderDirection;
  limit: number;
} & Abortable;

export const mapMessagesByAddressVariables = ({
  neuron,
  timestampFrom,
  offset = 0,
  types = [],
  orderDirection = 'desc',
  limit,
}: MessagesByAddressVariables) => ({
  address: `{${neuron}}`,
  limit,
  timestamp_from: numberToUtcDate(timestampFrom),
  offset,
  types: `{${types.map((t) => `"${t}"`).join(' ,')}}`,
  order_direction: orderDirection,
});

const fetchTransactions = async ({
  neuron,
  timestampFrom,
  offset = 0,
  types = [],
  orderDirection = 'desc',
  limit,
  abortSignal,
}: MessagesByAddressVariables) => {
  try {
    const res = await createIndexerClient(abortSignal).request<
      MessagesByAddressSenseQuery,
      MessagesByAddressSenseQueryVariables
    >(
      MessagesByAddressSenseDocument,
      mapMessagesByAddressVariables({
        neuron,
        timestampFrom,
        offset,
        types,
        orderDirection,
        limit,
        abortSignal,
      }) as MessagesByAddressSenseQueryVariables
    );

    return res?.messages_by_address as Transaction[];
  } catch (e) {
    console.error('fetchTransactions failed:', e);
    return [];
  }
};

export const fetchTransactionMessagesCount = async (
  address: NeuronAddress,
  timestampFrom: number,
  abortSignal: AbortSignal
) => {
  try {
    const res = await createIndexerClient(abortSignal).request<
      MessagesByAddressCountQuery,
      MessagesByAddressCountQueryVariables
    >(MessagesByAddressCountDocument, {
      address: `{${address}}`,
      timestamp: numberToUtcDate(timestampFrom),
    });

    return res?.messages_by_address_aggregate.aggregate?.count;
  } catch (e) {
    console.error('fetchTransactionMessagesCount failed:', e);
    return 0;
  }
};

export const fetchTransactionsIterable = ({
  neuron,
  timestampFrom,
  types,
  orderDirection,
  limit,
  abortSignal,
}: MessagesByAddressVariables) =>
  fetchIterableByOffset(fetchTransactions, {
    neuron,
    timestampFrom,
    types,
    orderDirection,
    limit,
    abortSignal,
  });
