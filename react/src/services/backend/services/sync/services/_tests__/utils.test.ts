import {
  MSG_MULTI_SEND_TRANSACTION_TYPE,
  MSG_SEND_TRANSACTION_TYPE,
} from 'src/services/backend/services/indexer/types';
import { extractSenseChats } from '../utils/sense';

describe('createSenseChats', () => {
  it('should create sense chats correctly', () => {
    const myAddress = 'myAddress';
    const transactions = [
      {
        type: MSG_MULTI_SEND_TRANSACTION_TYPE,
        transaction: {
          memo: 'memo1',
          block: {
            timestamp: '2022-01-01',
          },
        },
        value: {
          inputs: [{ address: 'addressMultiIn1' }, { address: 'addressMultiIn2' }],
          outputs: [{ address: 'address3' }, { address: 'myAddress' }],
        },
      },
      {
        type: MSG_MULTI_SEND_TRANSACTION_TYPE,
        transaction: {
          memo: 'memo2',
          block: {
            timestamp: '2022-01-01',
          },
        },
        value: {
          inputs: [{ address: 'myAddress' }, { address: 'addressN' }],
          outputs: [{ address: 'addressMultiOut1' }, { address: 'addressMultiOut2' }],
        },
      },
      {
        type: MSG_SEND_TRANSACTION_TYPE,
        transaction: {
          memo: 'memo3',
          block: {
            timestamp: '2022-01-10',
          },
        },
        value: {
          from_address: 'myAddress',
          to_address: 'address4',
        },
      },
      {
        type: MSG_SEND_TRANSACTION_TYPE,
        transaction: {
          memo: 'memo4',
          block: {
            timestamp: '2022-01-01',
          },
        },
        value: {
          from_address: 'address4',
          to_address: 'myAddress',
        },
      },
    ];

    const result = extractSenseChats(myAddress, transactions);
    expect([
      'addressMultiIn1',
      'addressMultiIn2',
      'addressMultiOut1',
      'addressMultiOut2',
      'address4',
    ]).toEqual(Array.from(result.keys()));
    expect(result.get('addressMultiIn1')?.transactions[0].transaction.memo).toEqual(
      transactions[0].transaction.memo
    );
    expect(result.get('address4')?.transactions.length).toEqual(2);
  });
});
