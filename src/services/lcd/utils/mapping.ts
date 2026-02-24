import { fromBase64 } from '@cosmjs/encoding';
import { MsgMultiSend, MsgSend } from 'cosmjs-types/cosmos/bank/v1beta1/tx';
import { Tx } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import {
  MSG_MULTI_SEND_TRANSACTION_TYPE,
  MSG_SEND_TRANSACTION_TYPE,
} from 'src/services/backend/services/indexer/types';
import { TransactionDto } from 'src/services/CozoDb/types/dto';
import { NeuronAddress } from 'src/types/base';
import { getNowUtcNumber } from 'src/utils/date';

// eslint-disable-next-line import/no-unused-modules
export const extractTxData = (data: string) => {
  const result = Tx.decode(fromBase64(data));
  const memo = result.body?.memo;
  const messages = result.body?.messages
    .map((message) => {
      const msgType = message.typeUrl.slice(1);
      if (msgType === MSG_SEND_TRANSACTION_TYPE) {
        return MsgSend.decode(message.value);
      }

      if (msgType === MSG_MULTI_SEND_TRANSACTION_TYPE) {
        return MsgMultiSend.decode(message.value);
      }
      return undefined;
    })
    .filter((message) => message !== undefined);

  return { memo, messages };
};

// eslint-disable-next-line import/no-unused-modules
export const mapWebsocketTxToTransactions = (neuron: NeuronAddress, result: any) => {
  const { data, events } = result;

  const hash = events['tx.hash'][0];
  const transactionType = events['message.action'][0].slice(1);
  const timestamp = getNowUtcNumber();
  const blockHeight = events['tx.height'][0];

  const { memo = '', messages } = extractTxData(data.value.TxResult.tx);

  const transactions: TransactionDto[] = [];
  messages!.forEach((message, index) => {
    transactions.push({
      hash,
      index,
      type: transactionType,
      timestamp,
      success: true,
      value: message!,
      memo,
      neuron,
      blockHeight,
    });
  });

  return transactions;
};
