import { Coin } from 'cosmjs-types/cosmos/base/v1beta1/coin';
import { SenseChat } from 'src/services/backend/types/sense';
import { TransactionDto } from 'src/services/CozoDb/types/dto';
import { NeuronAddress } from 'src/types/base';

import {
  MSG_MULTI_SEND_TRANSACTION_TYPE,
  MSG_SEND_TRANSACTION_TYPE,
  MsgSendTransaction,
} from '../../../indexer/types';

export const extractSenseChats = (myAddress: NeuronAddress, transactions: TransactionDto[]) => {
  const sendTransactions =
    transactions!.filter(
      (t) => t.type === MSG_SEND_TRANSACTION_TYPE || t.type === MSG_MULTI_SEND_TRANSACTION_TYPE
    ) || [];

  if (sendTransactions.length === 0) {
    return [];
  }
  const chats = new Map<NeuronAddress, SenseChat>();
  transactions.forEach((t) => {
    let userAddress = '';
    if (t.type === MSG_MULTI_SEND_TRANSACTION_TYPE) {
      const { inputs, outputs } = t.value;
      const isSender = inputs.find((i) => i.address === myAddress);
      const userMessages = isSender ? outputs : inputs;
      userMessages.forEach((msg) => updateSenseChat(chats, msg.address, t, msg.coins, isSender));
    } else if (t.type === MSG_SEND_TRANSACTION_TYPE) {
      const { fromAddress, toAddress, amount } = t.value as MsgSendTransaction['value'];
      const isSender = fromAddress === myAddress;
      userAddress = isSender ? toAddress : fromAddress;
      updateSenseChat(chats, userAddress, t, amount, isSender);
    }
  });

  return chats;
};

const updateSenseChat = (
  chats: Map<NeuronAddress, SenseChat>,
  addr: string,
  t: TransactionDto,
  amount: Coin[],
  isSender: boolean
): Map<string, SenseChat> => {
  const chat = chats.get(addr);
  const transactions = chat?.transactions || [];

  transactions.push(t);
  chats.set(addr, {
    userAddress: addr,
    lastSendTimestamp: isSender ? t.timestamp : chat?.lastSendTimestamp || 0,
    last: { amount, memo: t.memo, direction: isSender ? 'to' : 'from' },
    transactions,
  });
  return chats;
};
